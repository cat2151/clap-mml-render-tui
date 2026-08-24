//! chord mode の画面側ロジック。
//!
//! ドメイン（[`crate::state::chord`]）が「与えられた進行をどう鳴らすか」を持つのに対し、
//! ここは「どの進行と Key を引くか」「和音用の patch をどう当てるか」を持つ。

use std::time::Instant;

use cmrt_chord::{ChordProgressionCatalog, ChordVoicing};

use crate::{
    log_line, ChordPlayback, CycleRandomItem, FixedChordProgression, GridSequencerContext,
    GridSequencerScreen, CHORD_ROW,
};

/// 1回の抽選で、変換できる進行を探すために引き直す回数。
/// カタログの進行はすべて変換できるのが正常なので、これは保険。
const PICK_ATTEMPTS: usize = 32;

const CATALOG_UNAVAILABLE: &str = "コード進行データがありません";
const PATCHES_UNAVAILABLE: &str = "patch 一覧を読み込み中です";
const CHORD_PATCH_UNAVAILABLE: &str =
    "条件に合う patch がありません（カテゴリ設定 / cmrt build-voicing-cache）";

impl GridSequencerScreen {
    /// chord mode を on/off する。
    ///
    /// on にするときは、和音が潰れないよう先に patch を当ててから進行を引く。
    /// どちらかが失敗したら on にせず、ステータス行に理由を出す。
    pub(crate) fn toggle_chord_mode(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        // chord on/offでvisible row mappingが変わる前に、開始laneへlockされたgestureを閉じる。
        self.cancel_mouse_gesture();
        if self.state.chord().is_some() {
            let note_offs = self.state.set_chord(None, now);
            self.send_scheduled(&note_offs);
            self.chord_error = None;
            self.chord_enabled = false;
            self.pending_chord = false;
            // 音色ロードを伴わない切替なので、音量差はここで戻す。
            self.apply_chord_gains();
            log_line("grid-sequencer: chord off");
            return;
        }
        if self.enable_chord_mode(now, ctx) {
            self.prepare_connection();
        }
    }

    /// chord mode を on にする。和音の行へ patch を当て、進行を引く。
    ///
    /// 音色ロード（`prepare_connection()`）は呼ばない。復元時は初回のロードへ相乗り
    /// させたいので、いつロードするかは呼び出し側が決める。
    pub(crate) fn enable_chord_mode(
        &mut self,
        now: Instant,
        ctx: &GridSequencerContext<'_>,
    ) -> bool {
        let patch = match self.pick_chord_patch(ctx) {
            Ok(patch) => patch,
            Err(error) => {
                self.chord_error = Some(error.to_string());
                log_line(&format!("grid-sequencer: chord on rejected reason={error}"));
                return false;
            }
        };
        if let Some(fixed) = self.fixed_chord.clone() {
            let seed = self.state.chord().and_then(ChordPlayback::current_voicing);
            let playback = match fixed_chord_playback(fixed.input(), seed.as_ref()) {
                Ok(playback) => playback,
                Err(error) => {
                    self.chord_error = Some(error.clone());
                    log_line(&format!(
                        "grid-sequencer: fixed chord restore rejected reason={error}"
                    ));
                    return false;
                }
            };
            self.apply_chord_playback(playback, now, "fixed");
        } else if !self.reroll_chord(now, ctx) {
            return false;
        }
        self.state.instances_mut()[CHORD_ROW].patch = Some(patch);
        // bass / アルペジオ / drum は引けなくても chord mode は続ける
        // （直前の音色のまま鳴るだけ）。
        self.apply_dedicated_patches(ctx);
        self.chord_enabled = true;
        true
    }

    /// 1行入力から固定進行を即時適用する。失敗時は演奏・patch・固定設定を変更しない。
    pub(crate) fn apply_fixed_chord_input(
        &mut self,
        input: &str,
        now: Instant,
        ctx: &GridSequencerContext<'_>,
    ) -> Result<(), String> {
        let seed = self.state.chord().and_then(ChordPlayback::current_voicing);
        let playback = fixed_chord_playback(input, seed.as_ref())?;
        let was_off = self.state.chord().is_none();
        let chord_patch = if was_off {
            Some(self.pick_chord_patch(ctx).map_err(str::to_string)?)
        } else {
            None
        };

        self.cancel_mouse_gesture();
        self.cancel_cycle_swap_preserving_drain();
        self.apply_chord_playback(playback, now, "fixed-input");
        if let Some(patch) = chord_patch {
            self.state.instances_mut()[CHORD_ROW].patch = Some(patch);
            self.apply_dedicated_patches(ctx);
        }
        self.chord_enabled = true;
        self.pending_chord = false;
        self.fixed_chord = Some(FixedChordProgression::new(input.trim()));
        self.set_cycle_random(CycleRandomItem::Chord, false);
        self.chord_error = None;
        if was_off {
            self.prepare_connection();
        }
        Ok(())
    }

    /// セッションから復元した chord mode を、patch 一覧が揃ってから適用する。
    ///
    /// 毎フレーム呼ぶ。読み込み中は待ち、揃ったら成否によらず1回だけ試して予約を下ろす
    /// （失敗のたびに引き直すと、理由が変わらないまま log を溢れさせるだけになる）。
    ///
    /// on にできたときだけ true を返す。音色ロードは呼び出し側（`refresh_context`）が
    /// 他の行の割り当てとまとめて1回だけ走らせる。
    ///
    /// 失敗しても `chord_enabled`（＝セッションへ保存する値）は下ろさない。カタログの
    /// 取得失敗など一時的な理由のことがあり、ユーザーが選んだ設定を黙って捨てないため。
    pub(crate) fn poll_pending_chord(
        &mut self,
        now: Instant,
        ctx: &GridSequencerContext<'_>,
    ) -> bool {
        if !self.pending_chord || self.state.chord().is_some() || ctx.patches_are_loading() {
            return false;
        }
        self.pending_chord = false;
        let applied = self.enable_chord_mode(now, ctx);
        log_line(&format!("grid-sequencer: chord restore applied={applied}"));
        applied
    }

    /// 進行と Key だけを引き直して即座に差し替える。音色はそのまま。
    /// 成功したかどうかを返す。
    pub(crate) fn reroll_chord(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) -> bool {
        // 即座に差し替わるので、接続相手はいま鳴っているコード。
        let seed = self.state.chord().and_then(ChordPlayback::current_voicing);
        let Some(playback) = pick_chord(ctx.chord_catalog, seed.as_ref()) else {
            self.chord_error = Some(CATALOG_UNAVAILABLE.to_string());
            let note_offs = self.state.set_chord(None, now);
            self.send_scheduled(&note_offs);
            log_line("grid-sequencer: chord reroll failed reason=catalog-empty");
            return false;
        };
        self.apply_chord_playback(playback, now, "random");
        true
    }

    fn apply_chord_playback(&mut self, playback: ChordPlayback, now: Instant, source: &str) {
        if playback.max_voice_count() > crate::CHORD_VOICE_LANES {
            log_line(&format!(
                "grid-sequencer: chord voices truncated for multi-lane instances max={} limit={}",
                playback.max_voice_count(),
                crate::CHORD_VOICE_LANES,
            ));
        }
        log_line(&format!(
            "grid-sequencer: chord set source={source} key={} degrees={} chords={}",
            playback.key(),
            playback.degrees(),
            playback.chord_count(),
        ));
        let note_offs = self.state.set_chord(Some(playback), now);
        self.send_scheduled(&note_offs);
        self.chord_error = None;
    }

    /// 次サイクルを抽選し、差し替え待ちとして預ける。まだ鳴らさない。
    ///
    /// 何を引き直すかは [`crate::CycleRandom`] が決める。PATCH が ON の周だけ待機 bank
    /// への音色ロードが要るので、そのときだけ bank を切り替える差し替えとして預ける
    /// （OFF なら現在 bank 上で境界時に取り込むだけで済み、演奏も途切れない）。
    /// 抽選できたら true を返す。
    pub(crate) fn stage_next_cycle(
        &mut self,
        _now: Instant,
        ctx: &GridSequencerContext<'_>,
    ) -> bool {
        let policy = self.cycle_random;
        if !policy.chord && !policy.instances_change() {
            // 引き直すものが何も無い。差し替えずに同じ周をもう1度鳴らす。
            return true;
        }
        let Some(playback) = self.pick_next_playback(ctx) else {
            return false;
        };
        // 鳴っている grid はそのままに、複製の上で引き直す。差し替えは小節境界まで待つ。
        let mut instances = self.state.instances().to_vec();
        let patterns = self.state.draw_pattern_combination(policy, true);
        let drawn =
            crate::randomize_instance_slice(&mut instances, &[], policy, Some(&playback), patterns);
        if policy.patch {
            self.apply_random_patches_to(&mut instances, ctx);
        }
        log_line(&format!(
            "grid-sequencer: cycle staged random={} key={} degrees={} chords={} instances={}",
            policy.compact_label(),
            playback.key(),
            playback.degrees(),
            playback.chord_count(),
            instances.len(),
        ));
        if policy.patch {
            self.state
                .stage_next_cycle_with_drawn(instances, playback, drawn);
        } else {
            self.state
                .stage_next_cycle_in_place_with_drawn(instances, playback, drawn);
        }
        self.chord_error = None;
        true
    }

    /// 次サイクルで鳴らす進行。CHORD が OFF なら同じ進行を頭から続投する。
    ///
    /// 引けなかったときだけ `None`。差し替えそのものを見送る合図になる。
    fn pick_next_playback(&mut self, ctx: &GridSequencerContext<'_>) -> Option<ChordPlayback> {
        if !self.cycle_random.chord {
            return self.state.chord().map(ChordPlayback::restarted);
        }
        // 差し替えは小節境界。その直前に鳴っているのは現在の進行の最後のコード。
        let seed = self.state.chord().and_then(ChordPlayback::last_voicing);
        let playback = pick_chord(ctx.chord_catalog, seed.as_ref());
        if playback.is_none() {
            self.chord_error = Some(CATALOG_UNAVAILABLE.to_string());
            log_line("grid-sequencer: cycle stage failed reason=catalog-empty");
        }
        playback
    }

    /// chord mode 中の `r` / `R` に追随する。進行と Key を引き直し、
    /// `repick_patch` なら和音の行の音色も当て直す。
    /// drum 行は chord mode の外にも居るので、進行の引き直しとは切り離して当て直す。
    pub(crate) fn rechord_after_randomize(
        &mut self,
        now: Instant,
        ctx: &GridSequencerContext<'_>,
        repick_patch: bool,
    ) {
        if self.state.chord().is_some() && self.fixed_chord.is_none() {
            self.reroll_chord(now, ctx);
        }
        if repick_patch {
            self.apply_random_patches(ctx, false);
        }
    }

    /// 和音に使える patch を1つ引く。カテゴリと poly 判定の両方を満たすものだけが当たり。
    fn pick_chord_patch(&self, ctx: &GridSequencerContext<'_>) -> Result<String, &'static str> {
        if ctx.patches().is_empty() {
            return Err(PATCHES_UNAVAILABLE);
        }
        let candidates = self.patch_candidates_for_purpose(crate::GridPatchPurpose::Chord, ctx);
        let index =
            cmrt_tui_core::random::random_index(candidates.len()).ok_or(CHORD_PATCH_UNAVAILABLE)?;
        Ok(candidates[index].clone())
    }
}

/// 進行を1つ抽選し、auto voicing を通してから再生状態へ組み立てる。
///
/// auto voicing はここだけを通る（mode 切り替えは無く、常に効く）。`seed` は
/// 「この進行の1コード目が接続する相手」。cycle をまたいで引き直すとき、いま鳴って
/// いるコードを渡すと境界の top note の跳躍も最小化される。
fn pick_chord(
    catalog: &ChordProgressionCatalog,
    seed: Option<&ChordVoicing>,
) -> Option<ChordPlayback> {
    let pick = catalog.pick_playable(PICK_ATTEMPTS)?;
    let voicings = cmrt_chord::auto_voice(&pick.chords, seed);
    let (top_jump, bass_jump) = cmrt_chord::max_jumps(&voicings);
    // 接続相手との境界も1つの進行として測る。ここが跳ねていたら seed が効いていない。
    let bridge_top_jump = match (seed, voicings.first()) {
        (Some(seed), Some(first)) => cmrt_chord::max_jumps(&[seed.clone(), first.clone()])
            .0
            .to_string(),
        _ => "-".to_string(),
    };
    log_line(&format!(
        "grid-sequencer: auto voicing key={} degrees={} top_max_jump={top_jump} bass_max_jump={bass_jump} bridge_top_jump={bridge_top_jump}",
        pick.key,
        pick.degrees,
    ));
    ChordPlayback::from_voicings(pick.key, pick.degrees, voicings)
}

/// chord2mml-core の構造化parserから固定進行を作り、既存のauto voicingへ渡す。
fn fixed_chord_playback(input: &str, seed: Option<&ChordVoicing>) -> Result<ChordPlayback, String> {
    let parsed = cmrt_chord::parse_chord_progression(input)?;
    let voicings = cmrt_chord::auto_voice(parsed.chords(), seed);
    ChordPlayback::from_voicings(parsed.key_name(), parsed.chord_label(), voicings)
        .ok_or_else(|| "コード進行が空です".to_string())
}

#[cfg(test)]
mod tests;
