//! chord 行での `r` = コード進行の抽選。chord 行を丸ごと差し替え、**グリッドの小節数を
//! 進行の長さにそろえる**。
//!
//! 演奏 track の `r`（ランダム音色）と同じキーにしてあるのは、「その行の中身を
//! カタログから引き直す」という同じ操作だから。chord 行の中身は進行なので、引く先が
//! 音色カタログからコード進行カタログに変わるだけ。
//!
//! # 小節数を進行にそろえる
//!
//! chord wizard（`G`）は grid の幅を変えず、入りきらないコードを捨て、余った小節を
//! 空にする。こちらは逆に grid を進行に合わせる。4 コードなら 4 小節、16 コードなら
//! 16 小節。増えた小節には演奏 track の最後の小節を写し、減らすときに範囲外へ落ちる
//! 演奏 track のセルは失われる（数だけログに出す）。どちらも [`DawApp::set_measure_count`]。
//!
//! # `G` と分けているもの
//!
//! 演奏 track の init セルは触らない。chord 行の操作なので、どの track がその進行を
//! 鳴らすかは決めない（`G` はカーソル行を鳴らす側に回す）。既に chord 行から生成している
//! track があれば、そちらが新しい進行で鳴る。試聴もその track の音色で最初のコードを鳴らす。

use super::super::super::{DawApp, CHORD_TRACK};
use super::INIT_MEASURE;
use crate::mml::chord_generation::split_progression_into_measures;

/// chord 行の最初の演奏小節。row 全体を書き換えるので、書き終えたカーソルもここへ置く。
const FIRST_PLAY_MEASURE: usize = INIT_MEASURE + 1;

impl DawApp {
    /// chord 行の `r`: コード進行を抽選して chord 行に配り、小節数を進行の長さにする。
    pub(super) fn apply_random_chord_progression_to_chord_row(&mut self) {
        let degrees = match self.pick_playable_chord_progression() {
            Ok(degrees) => degrees,
            Err(message) => {
                self.append_log_line(message);
                return;
            }
        };
        let chords = split_progression_into_measures(&degrees);
        self.apply_random_chord_progression_with(&chords);
    }

    /// 抽選結果を受け取って書き込む本体。テストはこちらを直接呼ぶ。
    pub(crate) fn apply_random_chord_progression_with(&mut self, chords: &[String]) {
        if chords.is_empty() {
            self.append_log_line("コード進行を解釈できませんでした");
            return;
        }
        let measures = chords.len();
        let previous_measures = self.editor.measures;
        let dropped = self.set_measure_count(measures);
        if previous_measures != measures {
            self.append_log_line(format!(
                "random chord: 小節数を {previous_measures} → {measures} にしました"
            ));
        }
        if dropped > 0 {
            self.append_log_line(format!(
                "random chord: 小節を減らしたため、演奏 track の {dropped} セルを捨てました"
            ));
        }

        let mut written = false;
        for (chord, measure) in chords.iter().zip(FIRST_PLAY_MEASURE..) {
            written |= self.commit_insert_cell(CHORD_TRACK, measure, chord);
        }
        if !written && previous_measures == measures {
            // 既に同じ状態。何も変わらないなら preview も揺らさない。
            return;
        }
        self.editor.cursor_measure = FIRST_PLAY_MEASURE;

        self.save();
        self.sync_playback_mml_state();
        self.stop_play();
        // chord 行自体は音を持たない。chord 行から生成している track があれば、
        // その音色で最初のコードを鳴らす。
        if let Some(track) =
            super::preview_target_track(&self.editor.data, self.editor.tracks, CHORD_TRACK)
        {
            self.start_chord_wizard_realtime_preview(track, FIRST_PLAY_MEASURE);
        }
    }
}
