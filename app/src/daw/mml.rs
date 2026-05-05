//! DawApp の MML 構築・拍子/テンポ解析メソッド

use super::timing::{compute_measure_samples, parse_beat_numerator, parse_tempo_bpm};
use super::{DawApp, FIRST_PLAYABLE_TRACK};
use serde_json::{Map, Value};

// ─── 純粋関数（テスト用） ──────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct MmlFragment {
    json: Option<Value>,
    body: String,
}

impl MmlFragment {
    fn empty() -> Self {
        Self {
            json: None,
            body: String::new(),
        }
    }
}

fn split_mml_fragment(cell: &str) -> MmlFragment {
    use mmlabc_to_smf::mml_preprocessor;

    let cell = cell.trim();
    if cell.is_empty() {
        return MmlFragment::empty();
    }

    let preprocessed = mml_preprocessor::extract_embedded_json(cell);
    let json = preprocessed
        .embedded_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<Value>(json).ok());

    MmlFragment {
        json,
        body: preprocessed.remaining_mml.trim().to_string(),
    }
}

fn conductor_fragments(data: &[Vec<String>], num_measures: usize) -> Vec<MmlFragment> {
    (0..=num_measures)
        .filter_map(|measure| data[0].get(measure))
        .map(|cell| split_mml_fragment(cell))
        .collect()
}

fn merge_json_object(target: &mut Map<String, Value>, source: Map<String, Value>) {
    for (key, value) in source {
        match target.get_mut(&key) {
            Some(existing) => merge_json_value(existing, value),
            None => {
                target.insert(key, value);
            }
        }
    }
}

fn merge_json_value(target: &mut Value, source: Value) {
    match (target, source) {
        (Value::Object(target), Value::Object(source)) => merge_json_object(target, source),
        (Value::Array(target), Value::Array(source)) => target.extend(source),
        (target, source) => *target = source,
    }
}

fn merged_json_prefix(json_values: impl IntoIterator<Item = Value>) -> String {
    let mut merged = None::<Value>;
    for value in json_values {
        match &mut merged {
            Some(current) => merge_json_value(current, value),
            None => merged = Some(value),
        }
    }

    merged
        .and_then(|value| serde_json::to_string(&value).ok())
        .unwrap_or_default()
}

fn append_fragment_json_values<'a>(
    json_values: &mut Vec<Value>,
    fragments: impl IntoIterator<Item = &'a MmlFragment>,
) {
    json_values.extend(
        fragments
            .into_iter()
            .filter_map(|fragment| fragment.json.clone()),
    );
}

fn conductor_body(conductor: &[MmlFragment]) -> String {
    conductor
        .iter()
        .map(|fragment| fragment.body.as_str())
        .collect::<Vec<_>>()
        .join("")
}

fn track_non_json_branches(conductor: &str, init: &str, notes: &str) -> Vec<String> {
    if !notes.contains(';') {
        return vec![format!("{conductor}{init}{notes}")];
    }

    notes
        .split(';')
        .map(str::trim)
        .map(|part| {
            if part.is_empty() {
                String::new()
            } else {
                format!("{conductor}{init}{part}")
            }
        })
        .collect()
}

fn build_track_mml(conductor: &[MmlFragment], init: &MmlFragment, notes: &MmlFragment) -> String {
    let mut json_values = Vec::new();
    append_fragment_json_values(&mut json_values, conductor);
    append_fragment_json_values(&mut json_values, std::iter::once(init));
    append_fragment_json_values(&mut json_values, std::iter::once(notes));
    let json_prefix = merged_json_prefix(json_values);
    let conductor = conductor_body(conductor);
    let branches = track_non_json_branches(&conductor, &init.body, &notes.body);
    format!("{json_prefix}{}", branches.join(";"))
}

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
    let conductor = conductor_fragments(data, num_measures);
    let init = data
        .get(track)
        .and_then(|r| r.first())
        .map(|cell| split_mml_fragment(cell))
        .unwrap_or_else(MmlFragment::empty);
    let notes = data
        .get(track)
        .and_then(|r| r.get(measure))
        .map(|cell| split_mml_fragment(cell))
        .unwrap_or_else(MmlFragment::empty);
    build_track_mml(&conductor, &init, &notes)
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
    let conductor = conductor_fragments(data, num_measures);
    let conductor_body = conductor_body(&conductor);
    let solo_mode_active = solo_tracks.iter().any(|&is_solo| is_solo);

    let mut json_values = Vec::new();
    append_fragment_json_values(&mut json_values, &conductor);
    let mut track_branches = Vec::new();

    for t in FIRST_PLAYABLE_TRACK..tracks {
        if solo_mode_active && !solo_tracks.get(t).copied().unwrap_or(false) {
            continue;
        }
        let Some(notes_cell) = data.get(t).and_then(|row| row.get(measure)) else {
            continue;
        };
        if notes_cell.trim().is_empty() {
            continue;
        }

        let init = data
            .get(t)
            .and_then(|row| row.first())
            .map(|cell| split_mml_fragment(cell))
            .unwrap_or_else(MmlFragment::empty);
        let notes = split_mml_fragment(notes_cell);
        append_fragment_json_values(&mut json_values, [&init, &notes]);
        track_branches.extend(track_non_json_branches(
            &conductor_body,
            &init.body,
            &notes.body,
        ));
    }

    if track_branches.is_empty() {
        String::new()
    } else {
        format!(
            "{}{}",
            merged_json_prefix(json_values),
            track_branches.join(";")
        )
    }
}

impl DawApp {
    // ─── MML 構築 ─────────────────────────────────────────────

    pub(super) fn build_measure_track_mmls_for_measure(&self, measure: usize) -> Vec<String> {
        (0..self.tracks)
            .map(|track| {
                if track < FIRST_PLAYABLE_TRACK || !self.track_is_audible(track) {
                    String::new()
                } else {
                    let notes = self.data[track][measure].trim();
                    if notes.is_empty() {
                        String::new()
                    } else {
                        self.build_cell_mml(track, measure)
                    }
                }
            })
            .collect()
    }

    /// セル (track, measure) のレンダリング用 MML を構築する
    /// = merged JSON + track0 全体 + track[t][0] (音色/init) + track[t][m] (音符)
    /// 各セル先頭の JSON は最終 MML 先頭の 1 つの JSON にマージする。
    pub(super) fn build_cell_mml(&self, track: usize, measure: usize) -> String {
        build_cell_mml_from_data(&self.data, self.measures, track, measure)
    }

    /// 指定小節の全 track を結合した MML を構築する（1小節分の演奏用）
    /// track 0 はグローバルヘッダ（テンポ等）として各 track の先頭に付加するが、
    /// それ自体を独立した再生 track としては扱わない。
    /// 各セル先頭の JSON は最終 MML 先頭の 1 つの JSON にマージする。
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

    /// 全小節の per-track MML ベクターを構築する（演奏用）。
    /// index i → meas i+1, inner index t → track t の MML（再生しない track は空文字列）。
    pub(super) fn build_measure_track_mmls(&self) -> Vec<Vec<String>> {
        (1..=self.measures)
            .map(|measure| self.build_measure_track_mmls_for_measure(measure))
            .collect()
    }

    // ─── 拍子 / テンポ解析 ────────────────────────────────────

    /// track0 のマージ済み JSON から beat (拍子分子) を解析する。
    /// `{"beat": "4/4"}` → 4。解析できない場合は 4 (4/4デフォルト) を返す。
    /// 現バージョンでは 4/4 のみサポート。JSON は将来の拍子変更に備えた仮置き。
    pub(super) fn beat_numerator(&self) -> u32 {
        let conductor = conductor_fragments(&self.data, self.measures);
        let mut json_values = Vec::new();
        append_fragment_json_values(&mut json_values, &conductor);
        let merged_json = merged_json_prefix(json_values);
        parse_beat_numerator((!merged_json.is_empty()).then_some(merged_json.as_str()))
    }

    /// track0 MML から tempo (BPM) を解析する。
    /// `t120` → 120.0。解析できない場合は 120.0 (デフォルト)。[1.0, 960.0] にクランプ。
    pub(super) fn tempo_bpm(&self) -> f64 {
        let conductor = conductor_fragments(&self.data, self.measures);
        parse_tempo_bpm(&conductor_body(&conductor))
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
