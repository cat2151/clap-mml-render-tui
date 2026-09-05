//! meas ヘッダ行に「今どの小節を演奏しているか」を出す（playhead）。
//!
//! 表示は 3 つを重ねている。
//! 1. 現在小節のヘッダに背景色を敷く（どの列かが一目で分かる）
//! 2. その背景を拍単位で左から塗り進める（小節のどのへんかが分かる）
//! 3. ラベルの `M` を `>` に差し替える（色に頼らないフォールバック）
//!
//! 前景色は A-B マーカー（`A3` / `B7` / `AB3`）が既に使っているので、
//! 演奏位置は**背景色**で表す。両者は重ねても潰し合わない。

use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use super::super::super::{DawApp, DawPlayState};
use super::{cell_width, COLUMN_GAP};
use cmrt_tui_core::theme::{MONOKAI_BG, MONOKAI_CURSOR_BG, MONOKAI_PURPLE, MONOKAI_YELLOW};

#[cfg(test)]
mod tests;

/// 描画中の 1 フレームぶんの演奏位置。
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Playhead {
    /// 演奏中の小節 index（ヘッダの `M{n}` は `measure_index + 1`）。
    pub(super) measure_index: usize,
    /// セル本体のうち、左から塗り終えた桁数。
    pub(super) filled_columns: usize,
    /// preview か通常再生か。塗りの色だけが変わる。
    pub(super) state: DawPlayState,
}

/// 現在の演奏位置。停止中・位置未確定なら `None`。
///
/// 拍の割り出しには `PlayPosition::measure_duration`（その小節の実際の長さ）を使う。
/// app 側の tempo から計算すると、hot reload 直後にテンポがずれている間だけ
/// 塗りが小節末尾とずれてしまうため。
pub(super) fn playhead(app: &DawApp, measure_index_width: usize) -> Option<Playhead> {
    let state = *app.playback.play_state.lock().unwrap();
    if state == DawPlayState::Idle {
        return None;
    }
    let position = app.playback.position.lock().unwrap().clone()?;
    Some(Playhead {
        measure_index: position.measure_index,
        filled_columns: filled_columns(
            position.measure_start.elapsed().as_secs_f64(),
            position.measure_duration.as_secs_f64(),
            app.beat_numerator(),
            measure_index_width,
        ),
        state,
    })
}

/// 小節内の経過から、左から塗る桁数を求める。
///
/// 拍単位で段階的に進める（4/4・セル 4 桁なら 1 拍 1 桁の 4 段階）。
/// 拍子とセル桁数が割り切れない場合は、その拍を含む桁まで塗る。
/// 演奏が始まった時点で必ず 1 桁は塗る（0 桁だと現在小節が消えて見える）。
pub(super) fn filled_columns(
    elapsed_secs: f64,
    measure_duration_secs: f64,
    beat_count: u32,
    width: usize,
) -> usize {
    if width == 0 {
        return 0;
    }
    let beat_count = beat_count.max(1) as usize;
    if !measure_duration_secs.is_finite()
        || measure_duration_secs <= 0.0
        || !elapsed_secs.is_finite()
    {
        return width;
    }
    let ratio = (elapsed_secs / measure_duration_secs).clamp(0.0, 1.0);
    // 小節末尾ぴったり（ratio == 1.0）でも最終拍に留める。
    let beat_index = ((ratio * beat_count as f64) as usize).min(beat_count - 1);
    ((beat_index + 1) * width)
        .div_ceil(beat_count)
        .clamp(1, width)
}

/// 現在小節のヘッダラベル。`M3` → `>3`。
///
/// A-B マーカー（`A3` / `B7` / `AB3`）には `M` が無いのでそのまま返す。
/// A-B の情報を落とさないため、記号の付与より既存ラベルを優先する。
pub(super) fn header_label(label: &str) -> String {
    match label.strip_prefix('M') {
        Some(rest) => format!(">{rest}"),
        None => label.to_string(),
    }
}

/// 現在小節のヘッダ 1 列ぶん（セル本体 + 区切り空白）の span 列。
///
/// 塗り終えた桁は反転（背景 = 演奏色 / 前景 = 地の色）、残りは暗い背景。
/// 区切り空白は塗らない（列の切れ目を残す）。
pub(super) fn header_spans(
    label: &str,
    measure_index_in_header: usize,
    measure_width: usize,
    playhead: Playhead,
    base_style: Style,
) -> Vec<Span<'static>> {
    let width = cell_width(measure_index_in_header, measure_width);
    // 桁あふれで隣の列をずらさないよう、ここで切る。
    let cell: String = label.chars().take(width).collect();
    let cell = format!("{cell:<width$}");
    let filled_columns = playhead.filled_columns.min(width);
    let filled: String = cell.chars().take(filled_columns).collect();
    let rest: String = cell.chars().skip(filled_columns).collect();

    let fill_color = match playhead.state {
        DawPlayState::Preview => MONOKAI_PURPLE,
        _ => MONOKAI_YELLOW,
    };
    vec![
        Span::styled(
            filled,
            Style::default()
                .fg(MONOKAI_BG)
                .bg(fill_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(rest, base_style.bg(MONOKAI_CURSOR_BG)),
        Span::styled(" ".repeat(COLUMN_GAP), Style::default()),
    ]
}
