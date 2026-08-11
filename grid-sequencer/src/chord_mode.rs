//! chord mode の画面側ロジック。
//!
//! ドメイン（[`crate::state::chord`]）が「与えられた進行をどう鳴らすか」を持つのに対し、
//! ここは「どの進行と Key を引くか」「和音用の patch をどう当てるか」を持つ。

use std::time::Instant;

use cmrt_chord::{ChordProgressionCatalog, ChordVoicing};
use cmrt_surge_patches::{pick_for_role, PatchRole};

use crate::{
    log_line, ChordPlayback, GridSequencerContext, GridSequencerScreen, PatternEvolution,
    ARPEGGIO_ROW, BASS_ROW, CHORD_ROW,
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
        if !self.reroll_chord(now, ctx) {
            return false;
        }
        self.state.instances_mut()[CHORD_ROW].patch = Some(patch);
        // bass / アルペジオ / drum は引けなくても chord mode は続ける
        // （直前の音色のまま鳴るだけ）。
        self.apply_assigned_patches(ctx, false);
        self.chord_enabled = true;
        true
    }

    /// 用途が決まっている行（chord 行を除く）。
    ///
    /// chord 行は poly 判定が要るぶん扱いが違うので、この一覧には入れない。
    fn assigned_rows(&self) -> Vec<usize> {
        (0..self.state.instance_count())
            .filter(|row| self.row_patch_role(*row).is_some())
            .collect()
    }

    /// その行の patch の用途。用途が決まっていない行は `None`。
    ///
    /// drum 行は chord mode の on/off に関わらず用途が決まっている（打楽器の音でないと
    /// リズムにならない）。bass とアルペジオの行は chord mode 中だけの役目。
    fn row_patch_role(&self, row: usize) -> Option<PatchRole> {
        if let Some(drum) = self.state.drum_role(row) {
            return Some(crate::patch_role::drum_patch_role(drum));
        }
        self.state.chord()?;
        match row {
            BASS_ROW => Some(PatchRole::Bass),
            ARPEGGIO_ROW => Some(PatchRole::Arpeggio),
            _ => None,
        }
    }

    /// 用途が決まっている行へ、その用途の候補から patch を当てる。当てた行数を返す。
    ///
    /// `only_missing` が true なら、まだ音色の付いていない行だけを埋める。復元した音色や
    /// 手で選んだ音色を、patch 一覧が揃ったタイミングで奪わないため。
    pub(crate) fn apply_assigned_patches(
        &mut self,
        ctx: &GridSequencerContext<'_>,
        only_missing: bool,
    ) -> usize {
        let mut applied = 0;
        for row in self.assigned_rows() {
            if only_missing && self.state.instances()[row].patch.is_some() {
                continue;
            }
            applied += usize::from(self.apply_row_patch(row, ctx));
        }
        applied
    }

    /// 用途が決まっている行へ、その用途のカテゴリから引いた patch を当てる。
    /// 行が無い構成（track 数が足りない）では何もしない。当てられたら true。
    fn apply_row_patch(&mut self, row: usize, ctx: &GridSequencerContext<'_>) -> bool {
        let Some(patch) = self.pick_row_patch(row, ctx) else {
            return false;
        };
        let Some(instance) = self.state.instances_mut().get_mut(row) else {
            return false;
        };
        instance.patch = Some(patch);
        true
    }

    /// bass 行・アルペジオ行・drum 行に使える patch を1つ引く。
    ///
    /// どれも和音を1 instance へ重ねないので、chord 行と違い poly 判定は要求しない。
    fn pick_row_patch(&self, row: usize, ctx: &GridSequencerContext<'_>) -> Option<String> {
        let role = self.row_patch_role(row)?;
        pick_for_role(
            ctx.patches(),
            &ctx.role_filter(role).filter(),
            &ctx.poly_lookup(),
        )
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
        if playback.max_voice_count() > crate::CHORD_VOICE_LANES {
            log_line(&format!(
                "grid-sequencer: chord voices truncated for multi-lane instances max={} limit={}",
                playback.max_voice_count(),
                crate::CHORD_VOICE_LANES,
            ));
        }
        log_line(&format!(
            "grid-sequencer: chord set key={} degrees={} chords={}",
            playback.key(),
            playback.degrees(),
            playback.chord_count(),
        ));
        if playback.max_voice_count() > crate::CHORD_VOICE_LANES {
            log_line(&format!(
                "grid-sequencer: chord voices truncated for multi-lane instances max={} limit={}",
                playback.max_voice_count(),
                crate::CHORD_VOICE_LANES,
            ));
        }
        let note_offs = self.state.set_chord(Some(playback), now);
        self.send_scheduled(&note_offs);
        self.chord_error = None;
        true
    }

    /// 次サイクルを抽選し、差し替え待ちとして預ける。まだ鳴らさない。
    ///
    /// AUTO では進行・Key に加えて全instance/laneを引き直す。HOLD では進行・Keyだけを引き、
    /// 現在のlane patternを保ったままresolved noteだけを新しいコードへ追随させる。
    /// AUTO の音色ロードは待機 bank の裏で走らせる。HOLD は音色を保持するため
    /// ロードせず、現在 bank 上で進行だけを境界時に取り込む。
    /// 抽選できたら true を返す。
    pub(crate) fn stage_next_cycle(
        &mut self,
        _now: Instant,
        ctx: &GridSequencerContext<'_>,
    ) -> bool {
        // 差し替えは小節境界。その直前に鳴っているのは現在の進行の最後のコード。
        let seed = self.state.chord().and_then(ChordPlayback::last_voicing);
        let Some(playback) = pick_chord(ctx.chord_catalog, seed.as_ref()) else {
            self.chord_error = Some(CATALOG_UNAVAILABLE.to_string());
            log_line("grid-sequencer: cycle stage failed reason=catalog-empty");
            return false;
        };
        // 鳴っている grid はそのままに、複製の上で引き直す。差し替えは小節境界まで待つ。
        let mut instances = self.state.instances().to_vec();
        if self.pattern_evolution == PatternEvolution::Auto {
            crate::randomize_instance_slice(&mut instances, ctx.patches());
        }
        if self.pattern_evolution == PatternEvolution::Auto {
            // 和音の行だけは無差別抽選の結果を捨て、条件に合う patch へ当て直す。
            match self.pick_chord_patch(ctx) {
                Ok(patch) => instances[CHORD_ROW].patch = Some(patch),
                // 引けなくても chord mode は続ける（直前の patch のまま鳴らす）。
                Err(error) => {
                    self.chord_error = Some(error.to_string());
                    instances[CHORD_ROW].patch = self.state.instances()[CHORD_ROW].patch.clone();
                }
            }
            // bass・アルペジオ・drum の行も同じく、用途ごとのカテゴリから当て直す。
            for row in self.assigned_rows() {
                let Some(instance) = instances.get_mut(row) else {
                    continue;
                };
                instance.patch = self
                    .pick_row_patch(row, ctx)
                    .or_else(|| self.state.instances()[row].patch.clone());
            }
        }
        log_line(&format!(
            "grid-sequencer: cycle staged mode={} key={} degrees={} chords={} instances={}",
            self.pattern_evolution.label(),
            playback.key(),
            playback.degrees(),
            playback.chord_count(),
            instances.len(),
        ));
        match self.pattern_evolution {
            PatternEvolution::Auto => self.state.stage_next_cycle(instances, playback),
            PatternEvolution::Hold => self.state.stage_next_cycle_in_place(instances, playback),
        }
        self.chord_error = None;
        true
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
        if self.state.chord().is_some() {
            self.reroll_chord(now, ctx);
            if repick_patch {
                match self.pick_chord_patch(ctx) {
                    Ok(patch) => self.state.instances_mut()[CHORD_ROW].patch = Some(patch),
                    Err(error) => self.chord_error = Some(error.to_string()),
                }
            }
        }
        if repick_patch {
            self.apply_assigned_patches(ctx, false);
        }
    }

    /// 和音に使える patch を1つ引く。カテゴリと poly 判定の両方を満たすものだけが当たり。
    fn pick_chord_patch(&self, ctx: &GridSequencerContext<'_>) -> Result<String, &'static str> {
        if ctx.patches().is_empty() {
            return Err(PATCHES_UNAVAILABLE);
        }
        pick_for_role(
            ctx.patches(),
            &ctx.role_filter(PatchRole::Chord).filter(),
            &ctx.poly_lookup(),
        )
        .ok_or(CHORD_PATCH_UNAVAILABLE)
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

#[cfg(test)]
mod tests;
