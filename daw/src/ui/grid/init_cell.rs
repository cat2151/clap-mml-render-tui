//! init 列（meas 0）のセル表示文字列を組み立てる。
//!
//! init セルの実体は `{"Surge XT patch": "<表示パス>"}` という JSON 文字列で、
//! 先頭数文字を出すだけでは全 track が `{"Su` に見えてしまい、どの track が
//! どの音色なのか読み取れなかった。ここで `role:音色名` へ組み直す。
//!
//! 音色の読み解き（role と音色名）は mixer overlay と共通の
//! `ui::patch_display` に置いてある。ここはその結果を init 列の体裁へ整えるだけ。
//!
//! 表示だけの責務。セルの値そのものは書き換えない。

use cmrt_tui_core::patch_load::PatchCatalogSnapshot;

use super::super::super::DawApp;
use super::super::patch_display::{self, GENERATED_MARK, GENERATED_WITHOUT_DIRECTIVE};

#[cfg(test)]
mod tests;

/// track 0（Tempo 行）の init セル表示。`{"beat": "4/4"}t120` → `4/4 t120`。
///
/// 拍子が読めないときだけ `None`（＝呼び出し側は生の MML を出す）。
/// テンポは省略可（`t` が無ければ拍子だけを出す）。
fn tempo_label(mml: &str) -> Option<String> {
    let (value, phrase) = DawApp::extract_patch_json_and_phrase(mml)?;
    let beat = value.get("beat")?.as_str()?.trim();
    if beat.is_empty() {
        return None;
    }
    match crate::timing::parse_tempo_bpm(&phrase) {
        Some(bpm) => Some(format!("{beat} t{bpm}")),
        None => Some(beat.to_string()),
    }
}

/// 生成対象 track の init セル表示。音色名の頭に `*` を付ける。
///
/// 音色が書かれていない init セル（生成キーだけ）は音色名が無いので、
/// 代わりに chord2mml への指定を出す。`{"generate from chord track": ...}` の
/// JSON がそのまま見えるより読める。
fn generated_label(directive: &str, patch: Option<String>) -> String {
    match patch {
        Some(patch) => format!("{GENERATED_MARK}{patch}"),
        None => {
            let directive = directive.trim();
            let body = if directive.is_empty() {
                GENERATED_WITHOUT_DIRECTIVE
            } else {
                directive
            };
            format!("{GENERATED_MARK}{body}")
        }
    }
}

/// init セルに出す文字列。`None` なら呼び出し側が生の MML をそのまま詰める。
///
/// 桁詰め・切り詰めは行わない（列幅は `super::cell_width` の責務）。
pub(super) fn init_cell_text(
    track: usize,
    mml: &str,
    snapshot: Option<&PatchCatalogSnapshot>,
) -> Option<String> {
    match track {
        crate::tracks::TEMPO_TRACK => tempo_label(mml),
        // chord 行の init セルは chord2mml への指定文字列（`key:G` など）。
        // 音色でも拍子でもないので組み直さず、そのまま出す。
        crate::CHORD_TRACK => None,
        _ => {
            let display = patch_display::track_patch_display(mml, snapshot);
            match display.generated_directive() {
                // chord 行から生成される track。セルが空でも音が鳴るので、
                // その事実が init 列から読めるようにする。
                Some(directive) => Some(generated_label(directive, display.init_cell_label())),
                None => display.init_cell_label(),
            }
        }
    }
}

/// init 列のインジケータ行（セルの 1 行下）に出す文字列。
///
/// 生成対象 track だけ、chord2mml へ渡す指定（`close` / `key:G octave down` など）を
/// そのまま出す。init セル本体は 13 桁しかなく音色名で埋まるので、
/// ボイシング指定はもともと空いているこの行へ置く。
///
/// 桁詰め・切り詰めは行わない（列幅は `super::column_width` の責務）。
pub(super) fn init_indicator_text(track: usize, mml: &str) -> Option<String> {
    if track == crate::tracks::TEMPO_TRACK || track == crate::CHORD_TRACK {
        return None;
    }
    let directive = crate::mml::init_cell_chord_generation_label(mml)?;
    let directive = directive.trim();
    Some(if directive.is_empty() {
        GENERATED_WITHOUT_DIRECTIVE.to_string()
    } else {
        directive.to_string()
    })
}
