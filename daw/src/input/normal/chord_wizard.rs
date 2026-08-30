//! `G` キー = chord wizard。1 打鍵で「和音が鳴る状態」まで持っていく。
//!
//! # 1 コード = 1 小節
//!
//! grid は 1 セル = 1 小節で、chord 行もその 1 行。だから抽選した進行は
//! [`crate::mml::chord_generation::split_progression_into_measures`] で 1 コードずつに
//! 切ってから、**meas.1 から順に 1 小節へ 1 つずつ**配る。まるごと 1 セルへ書くと
//! 1 小節の中で等分され、時間軸が 1/N に圧縮される（これが最初の実装の誤り）。
//!
//! カーソル小節は見ない。row 全体を書き換える操作なので、**常に meas.1 から**始める。
//!
//! # 何を書くか
//!
//! 1. **chord 行の全体** ← N コードなら meas.1〜N へ 1 つずつ。
//!    **meas.N+1 以降は空にする**（前の進行の尻尾が残らないよう、row ごと差し替える）。
//! 2. **カーソル行の init セル** ← `"generate from chord track": "close"` を書き足す。
//!    既存のキー（音色・filter）は残す（[`crate::mml::init_cell_with_json_entries`]）。
//!    音色がまだ無い track にはランダム音色も同時に入れる。
//! 3. **カーソル行の meas.1〜N** ← 空にする。
//!    手書きのセルは chord 行より優先される（資料 4.5）ので、消さないと
//!    1〜2 を書いても音が変わらず「押したのに何も起きない」になる。
//!    消す前の内容は patch history へ退避する。
//!    **meas.N+1 以降は消さない。** そこには置き換える和音が無いので、消せば
//!    ただの破壊にしかならない。
//!
//! 書いたセルは全部で 1 操作なので、`u`（undo）もまとめて戻す
//! （[`crate::NormalCellUndo`] の並びで持つ）。
//!
//! # カーソルを meas.1 へ移す
//!
//! row 全体の操作なので、カーソルが meas.6 のままだと「画面が指している場所」と
//! 「wizard が書いた場所」がずれ、preview も進行より後ろの無音を鳴らしうる。
//! 移せば見ているものと鳴るものが一致する。
//!
//! # 抽選したものが鳴ることを確かめてから書く
//!
//! カタログには chord2mml が受け付けない degree 表記が混ざりうる。書いてから
//! 無音だと原因が分からないので、**この画面の生成経路そのもの**
//! （[`crate::mml::chord_generation::generate_mml_from_chord_cell`]）へ通して
//! 空文字にならないものだけを採る。chord 行 init に書かれた `key:` も込みで
//! 確かめられるので、key が壊れているときもここで落ちる。
//!
//! # 入力欄は作らない
//!
//! 進行の手入力は意図的にスコープ外（資料 4.9）。wizard は抽選専用。

use super::super::super::{DawApp, NormalCellUndo, CHORD_TRACK, FIRST_PLAYABLE_TRACK};
use super::super::PATCH_JSON_KEY;
use super::INIT_MEASURE;
use crate::mml::chord_generation::{
    generate_mml_from_chord_cell, split_progression_into_measures, GENERATE_FROM_CHORD_TRACK_KEY,
};
use cmrt_mml_overlay::line_play::{chord_line_events, LineProgram, LineStatus};
use cmrt_patches::PatchRole;

/// wizard が配り始める小節。row 全体を書き換えるので常にここから。
const FIRST_PLAY_MEASURE: usize = INIT_MEASURE + 1;

/// wizard が init セルへ書く chord2mml の指定。
///
/// `close`（密集ボイシング）を既定にしてある。ボイシングを変えたいときは
/// init セルの JSON を `i` で直接書き換える（専用キーは作らない。資料 4.7）。
const WIZARD_DIRECTIVE: &str = "close";

/// 鳴らない進行を引いたときの引き直し上限。
const PICK_ATTEMPTS: usize = 16;

struct ChordWizardRealtimePreview {
    patch: Option<String>,
    program: LineProgram,
}

impl DawApp {
    /// Daily DAW の空ページを、`G` を押した直後と同じ状態で開始する。
    ///
    /// コード進行カタログは app から構築後に注入されるため、DAW の load 時ではなく
    /// 注入後に呼ぶ。復帰したページの chord／演奏領域に何か 1 セルでもあれば、
    /// 既存内容とカーソルをそのまま保つ。
    pub fn populate_blank_daily_workspace(&mut self) {
        if self.workspace_kind != crate::WorkspaceKind::Daily
            || self.editor.tracks <= FIRST_PLAYABLE_TRACK
            || self.editor.measures < FIRST_PLAY_MEASURE
            || self
                .editor
                .data
                .iter()
                .skip(CHORD_TRACK)
                .flatten()
                .any(|cell| !cell.trim().is_empty())
        {
            return;
        }

        self.editor.cursor_track = FIRST_PLAYABLE_TRACK;
        self.editor.cursor_measure = FIRST_PLAY_MEASURE;
        self.apply_chord_wizard_to_current_measure();
    }

    /// `G`: コード進行を抽選し、カーソル行がそれを鳴らす状態にする。
    pub(super) fn apply_chord_wizard_to_current_measure(&mut self) {
        if self.editor.cursor_track < FIRST_PLAYABLE_TRACK {
            self.append_log_line("chord wizard は演奏トラックでのみ使用できます");
            return;
        }
        if self.cursor_play_measure_index().is_none() {
            self.append_log_line("chord wizard は init 以外の小節でのみ使用できます");
            return;
        }
        let degrees = match self.pick_playable_chord_progression() {
            Ok(degrees) => degrees,
            Err(message) => {
                self.append_log_line(message);
                return;
            }
        };
        // 音色が既にある track の音色は変えない（wizard は和音を足しに来ただけで、
        // 気に入って選んだ音色を黙って捨てるものではない）。
        let patch_name = match self.current_track_patch_name() {
            Some(_) => None,
            None => self.pick_random_patch_name_for_role(PatchRole::Chord),
        };
        if let Some(patch_name) = patch_name.as_deref() {
            self.append_log_line(random_patch_load_estimate_log_line(
                patch_name,
                self.catalog_patch_load_estimate_ms(patch_name),
            ));
        }

        self.apply_chord_wizard_with(&degrees, patch_name);
    }

    /// 抽選結果を受け取って書き込む本体。テストはこちらを直接呼ぶ。
    pub(crate) fn apply_chord_wizard_with(&mut self, degrees: &str, patch_name: Option<String>) {
        let track = self.editor.cursor_track;
        let chords = split_progression_into_measures(degrees);
        if chords.is_empty() {
            self.append_log_line(format!("コード進行を解釈できませんでした: {degrees}"));
            return;
        }
        // 小節数は定数ではなく実データの幅から取る（テストは短い grid を作る）。
        let measures = self.editor.data[CHORD_TRACK].len();
        // grid に入りきらないぶんは捨てる。既定の 8 小節より長い進行はカタログに
        // 無いが、増えたときに黙って index out of bounds にならないようにしておく。
        let filled = chords.len().min(measures - FIRST_PLAY_MEASURE);
        if filled < chords.len() {
            self.append_log_line(format!(
                "コード進行が grid に入りきらないため {} コードを省きました",
                chords.len() - filled
            ));
        }

        let mut json_entries = vec![(GENERATE_FROM_CHORD_TRACK_KEY, WIZARD_DIRECTIVE)];
        if let Some(patch_name) = patch_name.as_deref() {
            json_entries.push((PATCH_JSON_KEY, patch_name));
        }
        let next_init =
            crate::mml::init_cell_with_json_entries(&self.editor.data[track][0], &json_entries);

        // 手書きを消す前に退避する。init を書き換えたあとだと patch history の
        // 行き先（= その track の音色）が新しい音色に変わってしまう。
        for measure in FIRST_PLAY_MEASURE..FIRST_PLAY_MEASURE + filled {
            let handwritten = self.editor.data[track][measure].clone();
            self.record_current_measure_to_patch_history(&handwritten);
        }

        let mut writes = Vec::new();
        // chord 行は row ごと差し替える。前の進行のほうが長かったときに
        // 尻尾が残ると、カタログの進行と鳴るものが食い違う。
        for measure in FIRST_PLAY_MEASURE..measures {
            let text = chords
                .get(measure - FIRST_PLAY_MEASURE)
                .filter(|_| measure - FIRST_PLAY_MEASURE < filled)
                .cloned()
                .unwrap_or_default();
            writes.push((CHORD_TRACK, measure, text));
        }
        writes.push((track, INIT_MEASURE, next_init));
        // 和音を配った小節だけ手書きを消す。配っていない小節には置き換える和音が
        // 無いので、消せばただの破壊にしかならない。
        for measure in FIRST_PLAY_MEASURE..FIRST_PLAY_MEASURE + filled {
            writes.push((track, measure, String::new()));
        }
        let mut undo = Vec::new();
        for (write_track, write_measure, text) in writes {
            let previous = self.editor.data[write_track][write_measure].clone();
            if !self.commit_insert_cell(write_track, write_measure, &text) {
                continue;
            }
            undo.push(NormalCellUndo {
                track: write_track,
                measure: write_measure,
                previous,
                written: text,
            });
        }
        if undo.is_empty() {
            // 既に同じ状態。`g` と同じく、何も変わらないなら preview も揺らさない。
            return;
        }
        self.editor.cell_undo = Some(undo);

        // 書いた場所と画面が指す場所を合わせる（モジュール冒頭の「カーソルを
        // meas.1 へ移す」）。patch history への退避は済んでいるので、ここで
        // cursor_measure が動いても行き先は変わらない。
        self.editor.cursor_measure = FIRST_PLAY_MEASURE;

        self.save();
        self.sync_playback_mml_state();
        self.stop_play();
        self.start_chord_wizard_realtime_preview(track, FIRST_PLAY_MEASURE);
    }

    /// wizard が書いた最初のコードを、offline cache の進捗とは独立に一度だけ鳴らす。
    ///
    /// chord 行を直接 MML として解釈せず、MML overlay の `Ctrl+Space` と同じ変換へ
    /// chord init・演奏 track の directive・init MML を渡す。本番と同じ voicing にした
    /// うえで、音色だけは realtime server へ別途指定する。
    fn start_chord_wizard_realtime_preview(&self, track: usize, measure: usize) {
        let request = match self.chord_wizard_realtime_preview(track, measure) {
            Ok(request) => request,
            Err(error) => {
                self.append_log_line(format!("chord wizard: realtime preview error: {error}"));
                return;
            }
        };
        let Some(sender) = &self.mml_overlay_sender else {
            self.append_log_line(
                "chord wizard: realtime preview unavailable (play server is not initialized)",
            );
            return;
        };
        let command_id = sender.play_line(request.patch.as_deref(), request.program);
        self.append_log_line(format!(
            "chord wizard: realtime preview queued meas{} track{} command_id={command_id}",
            measure,
            crate::tracks::track_display_number(track),
        ));
    }

    fn chord_wizard_realtime_preview(
        &self,
        track: usize,
        measure: usize,
    ) -> Result<ChordWizardRealtimePreview, String> {
        let chord_init = self
            .editor
            .data
            .get(CHORD_TRACK)
            .and_then(|row| row.get(INIT_MEASURE))
            .ok_or_else(|| "chord init がありません".to_string())?;
        let chord = self
            .editor
            .data
            .get(CHORD_TRACK)
            .and_then(|row| row.get(measure))
            .ok_or_else(|| format!("meas{measure} がありません"))?;
        let track_init = self
            .editor
            .data
            .get(track)
            .and_then(|row| row.get(INIT_MEASURE))
            .ok_or_else(|| "演奏 track の init がありません".to_string())?;
        let directive = crate::mml::init_cell_chord_directive(track_init).unwrap_or_default();
        let mml_prefix = crate::mml::init_cell_mml_body(track_init);
        let (status, performance) = chord_line_events(chord, chord_init, &directive, &mml_prefix);
        match status {
            LineStatus::Played { .. } => Ok(ChordWizardRealtimePreview {
                patch: self.track_patch_name(track),
                program: LineProgram::once(performance),
            }),
            LineStatus::Idle => Err(format!("meas{measure} の chord が空です")),
            LineStatus::Error(error) => Err(error),
        }
    }

    /// コード進行を抽選する。実際に音になるものが出るまで引き直す。
    fn pick_playable_chord_progression(&self) -> Result<String, String> {
        let progressions = self.chord_progressions();
        if progressions.is_empty() {
            return Err("コード進行カタログが空です".to_string());
        }
        let chord_init = self.editor.data[CHORD_TRACK][INIT_MEASURE].clone();
        for _ in 0..PICK_ATTEMPTS {
            let Some(index) = cmrt_tui_core::random::random_index(progressions.len()) else {
                break;
            };
            let degrees = &progressions[index];
            // 実際に書くのは 1 小節 1 コードなので、進行まるごとではなく
            // **切り分けたあとの 1 コードずつ**が鳴ることを確かめる。
            let chords = split_progression_into_measures(degrees);
            if !chords.is_empty()
                && chords.iter().all(|chord| {
                    !generate_mml_from_chord_cell(&chord_init, WIZARD_DIRECTIVE, chord).is_empty()
                })
            {
                return Ok(degrees.clone());
            }
        }
        Err(format!(
            "コード進行を {PICK_ATTEMPTS} 回引きましたが、鳴るものがありませんでした"
        ))
    }

    /// 注入された供給元からコード進行の一覧を取る。未注入なら空。
    fn chord_progressions(&self) -> Vec<String> {
        match &self.chord_progression_source {
            Some(source) => source(),
            None => Vec::new(),
        }
    }
}

fn random_patch_load_estimate_log_line(patch_name: &str, estimate_ms: Option<u64>) -> String {
    let estimate = estimate_ms.map_or_else(
        || "unknown".to_string(),
        |milliseconds| format!("{:.3}s", milliseconds as f64 / 1_000.0),
    );
    format!("chord wizard: random patch load estimate={estimate} (catalog) patch={patch_name:?}")
}

#[cfg(test)]
mod tests;
