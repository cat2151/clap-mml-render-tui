//! DawApp の MML 構築・拍子/テンポ解析メソッド

use super::timing::{compute_measure_samples, parse_beat_numerator, parse_tempo_bpm};
use super::{DawApp, FIRST_PLAYABLE_TRACK};

// ─── 純粋関数（テスト用） ──────────────────────────────────────

/// data 配列からセル MML を構築する純粋関数。
///
/// `data[0][*]` がグローバルヘッダ（track0）、`data[track][0]` が音色、`data[track][measure]` が音符。
/// `build_cell_mml` と同じ MML を返すが、`DawApp` インスタンスを必要としないためテストで利用できる。
///
/// # 引数
/// - `data`: `data[track][measure]` の文字列スライス（`data[0]` が track0）
/// - `num_measures`: 小節数（`data[0].len() - 1`）
/// - `track`: 対象 track インデックス
/// - `measure`: 対象小節インデックス（0 = 音色列）
pub(super) fn build_cell_mml_from_data(
    data: &[Vec<String>],
    num_measures: usize,
    track: usize,
    measure: usize,
) -> String {
    use mmlabc_to_smf::mml_preprocessor;
    let track0: String = (0..=num_measures)
        .filter_map(|m| data[0].get(m))
        .enumerate()
        .map(|(m, cell)| {
            let cell = cell.trim();
            if m == 0 {
                mml_preprocessor::extract_embedded_json(cell).remaining_mml
            } else {
                cell.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("");
    let timbre = data
        .get(track)
        .and_then(|r| r.first())
        .map(|s| s.trim())
        .unwrap_or("");
    let notes = data
        .get(track)
        .and_then(|r| r.get(measure))
        .map(|s| s.trim())
        .unwrap_or("");
    build_track_mml(timbre, &track0, notes)
}

fn build_track_mml(timbre: &str, track0: &str, notes: &str) -> String {
    if !notes.contains(';') {
        return format!("{}{}{}", timbre, track0, notes);
    }

    notes
        .split(';')
        .map(str::trim)
        .map(|part| {
            if part.is_empty() {
                String::new()
            } else {
                format!("{}{}{}", timbre, track0, part)
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// data 配列から指定小節の演奏用 MML を構築する純粋関数。
///
/// 音符が 1 つもない小節は空文字列を返す。
pub(super) fn build_measure_mml_from_data(
    data: &[Vec<String>],
    num_measures: usize,
    tracks: usize,
    measure: usize,
    solo_tracks: &[bool],
) -> String {
    use mmlabc_to_smf::mml_preprocessor;

    let track0: String = (0..=num_measures)
        .map(|m| {
            let cell = data[0][m].trim();
            if m == 0 {
                mml_preprocessor::extract_embedded_json(cell).remaining_mml
            } else {
                cell.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("");
    let solo_mode_active = solo_tracks.iter().any(|&is_solo| is_solo);

    let track_mmls: Vec<String> = (FIRST_PLAYABLE_TRACK..tracks)
        .filter_map(|t| {
            if solo_mode_active && !solo_tracks.get(t).copied().unwrap_or(false) {
                return None;
            }
            let timbre = data[t][0].trim();
            let notes = data[t][measure].trim();
            if notes.is_empty() {
                None
            } else {
                Some(build_track_mml(timbre, &track0, notes))
            }
        })
        .collect();

    track_mmls.join(";")
}

impl DawApp {
    // ─── MML 構築 ─────────────────────────────────────────────

    /// セル (track, measure) のレンダリング用 MML を構築する
    /// = track[t][0] (音色) + track0 全体 + track[t][m] (音符)
    /// 音色 JSON を先頭に置くことで extract_embedded_json が正しく解析できる
    pub(super) fn build_cell_mml(&self, track: usize, measure: usize) -> String {
        build_cell_mml_from_data(&self.data, self.measures, track, measure)
    }

    /// 指定小節の全 track を結合した MML を構築する（1小節分の演奏用）
    /// track 0 はグローバルヘッダ（テンポ等）として各 track の先頭に付加するが、
    /// それ自体を独立した再生 track としては扱わない。
    /// 音色 JSON を先頭に置くことで extract_embedded_json が正しく解析できる
    pub(super) fn build_measure_mml(&self, measure: usize) -> String {
        build_measure_mml_from_data(
            &self.data,
            self.measures,
            self.tracks,
            measure,
            &self.solo_tracks,
        )
    }

    /// 全小節の per-measure MML ベクターを構築する（演奏用; hot reload に使用）
    /// index i → meas i+1 の MML（空小節は空文字列）
    pub(super) fn build_measure_mmls(&self) -> Vec<String> {
        (1..=self.measures)
            .map(|m| self.build_measure_mml(m))
            .collect()
    }

    // ─── 拍子 / テンポ解析 ────────────────────────────────────

    /// track0[0] の JSON から beat (拍子分子) を解析する。
    /// `{"beat": "4/4"}` → 4。解析できない場合は 4 (4/4デフォルト) を返す。
    /// 現バージョンでは 4/4 のみサポート。JSON は将来の拍子変更に備えた仮置き。
    pub(super) fn beat_numerator(&self) -> u32 {
        use mmlabc_to_smf::mml_preprocessor;
        let header = self.data[0][0].trim();
        let preprocessed = mml_preprocessor::extract_embedded_json(header);
        parse_beat_numerator(preprocessed.embedded_json.as_deref())
    }

    /// track0 MML から tempo (BPM) を解析する。
    /// `t120` → 120.0。解析できない場合は 120.0 (デフォルト)。[1.0, 960.0] にクランプ。
    pub(super) fn tempo_bpm(&self) -> f64 {
        use mmlabc_to_smf::mml_preprocessor;
        let track0: String = (0..=self.measures)
            .map(|m| self.data[0][m].trim())
            .collect::<Vec<_>>()
            .join("");
        let preprocessed = mml_preprocessor::extract_embedded_json(&track0);
        parse_tempo_bpm(&preprocessed.remaining_mml)
            .unwrap_or(120.0)
            .clamp(1.0, 960.0)
    }

    /// 1小節のサンプル数を計算する（ステレオ: L/R インターリーブ）。
    /// beat_numerator * (60 / bpm) * sample_rate * 2
    pub(super) fn measure_duration_samples(&self) -> usize {
        compute_measure_samples(
            self.beat_numerator(),
            self.tempo_bpm(),
            self.cfg.sample_rate,
        )
    }
}

#[cfg(test)]
#[path = "../tests/daw/mml.rs"]
mod tests;
