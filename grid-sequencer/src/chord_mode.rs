//! chord mode の画面側ロジック。
//!
//! ドメイン（[`crate::state::chord`]）が「与えられた進行をどう鳴らすか」を持つのに対し、
//! ここは「どの進行と Key を引くか」「和音用の patch をどう当てるか」を持つ。

use std::time::Instant;

use cmrt_chord::ChordProgressionCatalog;

use crate::{
    log_line, pick_chord_patch, ChordPlayback, GridSequencerContext, GridSequencerScreen, CHORD_ROW,
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
        self.state.rows_mut()[CHORD_ROW].patch = Some(patch);
        self.chord_enabled = true;
        true
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
        let Some(playback) = pick_chord(ctx.chord_catalog) else {
            self.chord_error = Some(CATALOG_UNAVAILABLE.to_string());
            let note_offs = self.state.set_chord(None, now);
            self.send_scheduled(&note_offs);
            log_line("grid-sequencer: chord reroll failed reason=catalog-empty");
            return false;
        };
        log_line(&format!(
            "grid-sequencer: chord set key={} degrees={} chords={}",
            playback.key(),
            playback.degrees(),
            playback.chord_count(),
        ));
        let note_offs = self.state.set_chord(Some(playback), now);
        self.send_scheduled(&note_offs);
        self.chord_error = None;
        true
    }

    /// 次サイクルを抽選し、差し替え待ちとして預ける。まだ鳴らさない。
    ///
    /// 進行・Key に加えて、**全行の音色・note・音長・セル**を引き直す（`r` キーと同じ範囲）。
    /// 音色ロードは待機 bank の裏で走らせるので、ここで演奏は止まらない。
    /// 抽選できたら true を返す。
    pub(crate) fn stage_next_cycle(
        &mut self,
        _now: Instant,
        ctx: &GridSequencerContext<'_>,
    ) -> bool {
        let Some(playback) = pick_chord(ctx.chord_catalog) else {
            self.chord_error = Some(CATALOG_UNAVAILABLE.to_string());
            log_line("grid-sequencer: cycle stage failed reason=catalog-empty");
            return false;
        };
        // 鳴っている grid はそのままに、複製の上で引き直す。差し替えは小節境界まで待つ。
        let mut rows = self.state.rows().to_vec();
        crate::randomize_row_slice(&mut rows, ctx.patches());
        crate::snap_rows_to_chord(&mut rows, &playback);
        // 和音の行だけは無差別抽選の結果を捨て、条件に合う patch へ当て直す。
        match self.pick_chord_patch(ctx) {
            Ok(patch) => rows[CHORD_ROW].patch = Some(patch),
            // 引けなくても chord mode は続ける（直前に当たっていた patch のまま鳴らす）。
            Err(error) => {
                self.chord_error = Some(error.to_string());
                rows[CHORD_ROW].patch = self.state.rows()[CHORD_ROW].patch.clone();
            }
        }
        log_line(&format!(
            "grid-sequencer: cycle staged key={} degrees={} chords={} rows={}",
            playback.key(),
            playback.degrees(),
            playback.chord_count(),
            rows.len(),
        ));
        self.state.stage_next_cycle(rows, playback);
        self.chord_error = None;
        true
    }

    /// chord mode 中の `r` / `R` に追随する。進行と Key を引き直し、
    /// `repick_patch` なら和音の行の音色も当て直す。
    pub(crate) fn rechord_after_randomize(
        &mut self,
        now: Instant,
        ctx: &GridSequencerContext<'_>,
        repick_patch: bool,
    ) {
        if self.state.chord().is_none() {
            return;
        }
        self.reroll_chord(now, ctx);
        if !repick_patch {
            return;
        }
        match self.pick_chord_patch(ctx) {
            Ok(patch) => self.state.rows_mut()[CHORD_ROW].patch = Some(patch),
            Err(error) => self.chord_error = Some(error.to_string()),
        }
    }

    /// 和音に使える patch を1つ引く。カテゴリと poly 判定の両方を満たすものだけが当たり。
    fn pick_chord_patch(&self, ctx: &GridSequencerContext<'_>) -> Result<String, &'static str> {
        if ctx.patches().is_empty() {
            return Err(PATCHES_UNAVAILABLE);
        }
        pick_chord_patch(ctx.patches(), ctx.voicing, ctx.chord_patch_categories)
            .ok_or(CHORD_PATCH_UNAVAILABLE)
    }
}

fn pick_chord(catalog: &ChordProgressionCatalog) -> Option<ChordPlayback> {
    let pick = catalog.pick_playable(PICK_ATTEMPTS)?;
    ChordPlayback::new(pick.key, pick.degrees, pick.chords)
}

#[cfg(test)]
mod tests;
