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
            // 音色ロードを伴わない切替なので、音量差はここで戻す。
            self.apply_chord_gains();
            log_line("grid-sequencer: chord off");
            return;
        }
        let patch = match self.pick_chord_patch(ctx) {
            Ok(patch) => patch,
            Err(error) => {
                self.chord_error = Some(error.to_string());
                log_line(&format!("grid-sequencer: chord on rejected reason={error}"));
                return;
            }
        };
        if !self.reroll_chord(now, ctx) {
            return;
        }
        self.state.rows_mut()[CHORD_ROW].patch = Some(patch);
        self.prepare_connection();
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

    /// 進行を1周し終えたときの引き直し。進行・Key に加えて**全行の音色**も引き直す。
    ///
    /// 音色ロード（`prepare()`）が `stop_live_all()` を伴うので、この間だけ演奏が
    /// 止まる。進行の変わり目に音色も変わるので、区切りとしてはむしろ自然。
    pub(crate) fn reroll_chord_cycle(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        if !self.reroll_chord(now, ctx) {
            return;
        }
        let note_offs = self.state.randomize_patches(now, ctx.patches());
        self.send_scheduled(&note_offs);
        // 和音の行だけは無差別抽選の結果を捨て、条件に合う patch へ当て直す。
        match self.pick_chord_patch(ctx) {
            Ok(patch) => self.state.rows_mut()[CHORD_ROW].patch = Some(patch),
            // 引けなくても chord mode は続ける（直前に当たっていた patch のまま鳴らす）。
            Err(error) => self.chord_error = Some(error.to_string()),
        }
        log_line(&format!(
            "grid-sequencer: chord cycle reroll patches instances={}",
            self.track_count()
        ));
        self.prepare_connection();
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
