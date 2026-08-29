//! DawApp の MML 構築・拍子/テンポ解析メソッド

use super::timing::{compute_measure_samples, parse_beat_numerator, parse_tempo_bpm};
use super::{DawApp, CHORD_TRACK, FIRST_PLAYABLE_TRACK};
use serde_json::{Map, Value};

pub(super) mod chord_generation;

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

fn cell_text(data: &[Vec<String>], track: usize, measure: usize) -> &str {
    data.get(track)
        .and_then(|row| row.get(measure))
        .map(String::as_str)
        .unwrap_or_default()
}

/// init セルの JSON から `"generate from chord track"` の値を取り出す。
///
/// **キーが存在するかどうかだけ**がその track を chord 行から生成するかの判定。
/// 値は chord2mml へそのまま渡す任意の文字列で、cmrt 側では検証も語彙定義もしない。
/// 文字列以外が書かれていた場合は空文字扱い（キーがある以上、生成対象ではある）。
fn track_chord_directive(init: &MmlFragment) -> Option<&str> {
    let value = init
        .json
        .as_ref()?
        .get(chord_generation::GENERATE_FROM_CHORD_TRACK_KEY)?;
    Some(value.as_str().unwrap_or_default())
}

/// init セルの生の文字列から `"generate from chord track"` の値を取り出す。
///
/// 判定と表示（`ui::grid`）の両方がここを通る。**キー名の文字列を他所へ
/// 直書きしないこと**（綴りの二重管理になる）。
pub(super) fn init_cell_chord_directive(init_cell: &str) -> Option<String> {
    track_chord_directive(&split_mml_fragment(init_cell)).map(str::to_string)
}

/// init セルの JSON 以外の MML。chord 行から生成した MML の前にも本番と同じく付く。
pub(super) fn init_cell_mml_body(init_cell: &str) -> String {
    split_mml_fragment(init_cell).body
}

/// init セルへ JSON のキーを書き足した文字列を返す。
///
/// **既存のキーは残す**（音色や filter を消さずに `"generate from chord track"` だけを
/// 足せる）。同じキーがあれば上書きする。JSON が無い / 壊れている init セルには
/// 新しい JSON を作り、JSON のうしろに body があればそのまま残す。
///
/// キー名の綴りを呼び出し側が直書きしなくて済むよう、値は `&str` で受ける。
pub(super) fn init_cell_with_json_entries(init_cell: &str, entries: &[(&str, &str)]) -> String {
    let fragment = split_mml_fragment(init_cell);
    let mut object = match fragment.json {
        Some(Value::Object(object)) => object,
        // object 以外（配列や数値）が書かれていた場合は、生成キーを載せる器にならない。
        // 捨てて作り直す（この JSON では音色も読めていないので失うものが無い）。
        _ => Map::new(),
    };
    for (key, value) in entries {
        object.insert((*key).to_string(), Value::String((*value).to_string()));
    }
    format!("{}{}", Value::Object(object), fragment.body)
}

/// その track が chord 行から生成される設定になっているか（init セルに生成キーがあるか）。
///
/// measure セルの中身は見ない。init 列の表示のように「その track の設定」だけを
/// 知りたいときに使う。
pub(super) fn track_generates_from_chord_row(data: &[Vec<String>], track: usize) -> bool {
    track >= FIRST_PLAYABLE_TRACK && init_cell_chord_directive(cell_text(data, track, 0)).is_some()
}

/// そのセルの MML が chord 行から生成されるか（手書きが空 + init に生成キーがある）。
///
/// chord2mml を呼ばない安価な判定。chord 行を編集したときに、どのセルの
/// キャッシュを捨てるべきかを選ぶのに使う。
pub(super) fn cell_is_generated_from_chord_row(
    data: &[Vec<String>],
    track: usize,
    measure: usize,
) -> bool {
    if measure == 0 || !track_generates_from_chord_row(data, track) {
        return false;
    }
    cell_text(data, track, measure).trim().is_empty()
}

/// セル (track, measure) の音符 fragment を解決する。
///
/// 手書きのセルが空でなければ、それをそのまま使う（手書きが優先。生成が手書きを
/// 黙って上書きすることはない）。空のときだけ、init に `"generate from chord track"`
/// があれば chord 行の同じ measure から生成する。
fn resolve_notes_fragment(
    data: &[Vec<String>],
    track: usize,
    measure: usize,
    init: &MmlFragment,
) -> MmlFragment {
    let cell = cell_text(data, track, measure);
    if !cell.trim().is_empty() {
        return split_mml_fragment(cell);
    }
    if track < FIRST_PLAYABLE_TRACK || measure == 0 {
        return MmlFragment::empty();
    }
    let Some(directive) = track_chord_directive(init) else {
        return MmlFragment::empty();
    };
    MmlFragment {
        json: None,
        body: chord_generation::generate_mml_from_chord_cell(
            cell_text(data, CHORD_TRACK, 0),
            directive,
            cell_text(data, CHORD_TRACK, measure),
        ),
    }
}

/// セル (track, measure) が演奏される中身を持つか。
///
/// **手書きセルが空でも、chord 行から生成されるなら `true`。**
/// キャッシュ投入や再生対象の判定を生のセル文字列で行うと、生成されたセルが
/// 丸ごと無視されて音が出ない。
pub(super) fn cell_has_content(data: &[Vec<String>], track: usize, measure: usize) -> bool {
    let init = split_mml_fragment(cell_text(data, track, 0));
    resolve_notes_fragment(data, track, measure, &init) != MmlFragment::empty()
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
    let notes = resolve_notes_fragment(data, track, measure, &init);
    build_track_mml(&conductor, &init, &notes)
}

pub(super) fn measure_duration_samples_from_data(
    data: &[Vec<String>],
    num_measures: usize,
    sample_rate: f64,
) -> usize {
    let conductor = conductor_fragments(data, num_measures);
    let mut json_values = Vec::new();
    append_fragment_json_values(&mut json_values, &conductor);
    let merged_json = merged_json_prefix(json_values);
    let beat = parse_beat_numerator((!merged_json.is_empty()).then_some(merged_json.as_str()));
    let bpm = parse_tempo_bpm(&conductor_body(&conductor))
        .unwrap_or(120.0)
        .clamp(1.0, 960.0);
    compute_measure_samples(beat, bpm, sample_rate)
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
        let init = split_mml_fragment(cell_text(data, t, 0));
        let notes = resolve_notes_fragment(data, t, measure, &init);
        if notes == MmlFragment::empty() {
            continue;
        }

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

    /// overlay の preview 用に、conductor 行・chord 行・カーソル track だけを抜き出した
    /// 縮小グリッドを作る。
    ///
    /// **行の並びは本物のグリッドと同じにすること。** `build_cell_mml_from_data` は
    /// 行 index で行の役割（conductor / chord 行 / 演奏 track）を判断するので、
    /// 詰めて並べると別の役割の行として解釈される。
    pub(super) fn preview_grid_for_cursor_track(&self) -> Vec<Vec<String>> {
        let mut grid: Vec<Vec<String>> = (0..FIRST_PLAYABLE_TRACK)
            .map(|track| self.editor.data[track].clone())
            .collect();
        grid.push(self.editor.data[self.editor.cursor_track].clone());
        grid
    }

    pub(super) fn build_measure_track_mmls_for_measure(&self, measure: usize) -> Vec<String> {
        (0..self.editor.tracks)
            .map(|track| {
                if track < FIRST_PLAYABLE_TRACK || !self.track_is_audible(track) {
                    String::new()
                } else if cell_has_content(&self.editor.data, track, measure) {
                    self.build_cell_mml(track, measure)
                } else {
                    String::new()
                }
            })
            .collect()
    }

    /// セル (track, measure) のレンダリング用 MML を構築する
    /// = merged JSON + track0 全体 + track[t][0] (音色/init) + track[t][m] (音符)
    /// 各セル先頭の JSON は最終 MML 先頭の 1 つの JSON にマージする。
    pub(super) fn build_cell_mml(&self, track: usize, measure: usize) -> String {
        build_cell_mml_from_data(&self.editor.data, self.editor.measures, track, measure)
    }

    /// 指定小節の全 track を結合した MML を構築する（1小節分の演奏用）
    /// track 0 はグローバルヘッダ（テンポ等）として各 track の先頭に付加するが、
    /// それ自体を独立した再生 track としては扱わない。
    /// 各セル先頭の JSON は最終 MML 先頭の 1 つの JSON にマージする。
    pub(super) fn build_measure_mml(&self, measure: usize) -> String {
        build_measure_mml_from_data(
            &self.editor.data,
            self.editor.measures,
            self.editor.tracks,
            measure,
            &self.solo_tracks,
        )
    }

    /// 全小節の per-measure MML ベクターを構築する（演奏用; hot reload に使用）
    /// index i → meas i+1 の MML（空小節は空文字列）
    pub(super) fn build_measure_mmls(&self) -> Vec<String> {
        (1..=self.editor.measures)
            .map(|m| self.build_measure_mml(m))
            .collect()
    }

    /// 全小節の per-track MML ベクターを構築する（演奏用）。
    /// index i → meas i+1, inner index t → track t の MML（再生しない track は空文字列）。
    pub(super) fn build_measure_track_mmls(&self) -> Vec<Vec<String>> {
        (1..=self.editor.measures)
            .map(|measure| self.build_measure_track_mmls_for_measure(measure))
            .collect()
    }

    // ─── 拍子 / テンポ解析 ────────────────────────────────────

    /// track0 のマージ済み JSON から beat (拍子分子) を解析する。
    /// `{"beat": "4/4"}` → 4。解析できない場合は 4 (4/4デフォルト) を返す。
    /// 現バージョンでは 4/4 のみサポート。JSON は将来の拍子変更に備えた仮置き。
    pub(super) fn beat_numerator(&self) -> u32 {
        let conductor = conductor_fragments(&self.editor.data, self.editor.measures);
        let mut json_values = Vec::new();
        append_fragment_json_values(&mut json_values, &conductor);
        let merged_json = merged_json_prefix(json_values);
        parse_beat_numerator((!merged_json.is_empty()).then_some(merged_json.as_str()))
    }

    /// track0 MML から tempo (BPM) を解析する。
    /// `t120` → 120.0。解析できない場合は 120.0 (デフォルト)。[1.0, 960.0] にクランプ。
    pub(super) fn tempo_bpm(&self) -> f64 {
        let conductor = conductor_fragments(&self.editor.data, self.editor.measures);
        parse_tempo_bpm(&conductor_body(&conductor))
            .unwrap_or(120.0)
            .clamp(1.0, 960.0)
    }

    /// 1小節のサンプル数を計算する（ステレオ: L/R インターリーブ）。
    /// beat_numerator * (60 / bpm) * sample_rate * 2
    pub(super) fn measure_duration_samples(&self) -> usize {
        measure_duration_samples_from_data(
            &self.editor.data,
            self.editor.measures,
            self.cfg.sample_rate,
        )
    }
}

#[cfg(test)]
pub(crate) mod tests;
