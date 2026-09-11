//! degrees 1 行を span へ割るところ。**桁がずれないこと**が主題。
//!
//! 画面全体で見た反転の位置は `ui/tests/chord_highlight.rs`（描画 buffer を読む）。
//! ここでは切り方だけを、文字列として読む。

use super::*;

use cmrt_tui_core::status::base_style;

use crate::ui::text::{display_width, ELLIPSIS};

/// span を「文字列」と「表示幅」で読めるようにする。
fn texts(spans: &[Span<'static>]) -> Vec<String> {
    spans.iter().map(|span| span.content.to_string()).collect()
}

fn total_width(spans: &[Span<'static>]) -> usize {
    spans
        .iter()
        .map(|span| display_width(&span.content))
        .sum::<usize>()
}

/// 反転している span の中身（無ければ空）。
fn reversed(spans: &[Span<'static>]) -> String {
    spans
        .iter()
        .filter(|span| span.style.add_modifier.contains(Modifier::REVERSED))
        .map(|span| span.content.to_string())
        .collect()
}

/// 範囲を指すと、**その chord だけ**が反転した 3 つの span になる。
#[test]
fn the_pointed_chord_becomes_its_own_reversed_span() {
    let degrees = "I-V-VIm-IV";

    let spans = degrees_spans(degrees, 20, Some(2..3), base_style());

    assert_eq!(texts(&spans), vec!["I-", "V", "-VIm-IV          "]);
    assert_eq!(total_width(&spans), 20, "20 桁ちょうど（末尾は padding）");
    assert_eq!(reversed(&spans), "V");
}

/// 反転しても**桁は 1 つも動かない**（span を割っただけ）。
#[test]
fn splitting_never_changes_the_column_count() {
    let degrees = "I-V-VIm-IV";

    for index in 0..degrees.len() {
        let spans = degrees_spans(degrees, 20, Some(index..index + 1), base_style());
        assert_eq!(total_width(&spans), 20, "range={index}");
        assert_eq!(
            texts(&spans).concat(),
            fit_width(degrees, 20),
            "range={index}"
        );
    }
}

/// 範囲を渡さなければ span は 1 つのまま（写しがまだ届いていない行）。
#[test]
fn without_a_range_the_line_is_a_single_span() {
    let spans = degrees_spans("I-V", 10, None, base_style());

    assert_eq!(texts(&spans), vec!["I-V       "]);
    assert_eq!(reversed(&spans), "");
}

/// **全角が混ざっても桁がずれない。** 位置はバイトで来るが、割るのは表示幅。
#[test]
fn a_full_width_chord_is_cut_by_display_width_not_by_bytes() {
    // `♯` は 3 バイト。バイト位置のまま桁として使うと、ここで大きくずれる。
    let degrees = "C♯m7-F♯7-BM7";
    let f_sharp = degrees.find("F♯7").expect("2 つめの chord があること");

    let spans = degrees_spans(
        degrees,
        20,
        Some(f_sharp..f_sharp + "F♯7".len()),
        base_style(),
    );

    assert_eq!(reversed(&spans), "F♯7");
    let before = display_width(&spans[0].content);
    assert_eq!(
        before,
        display_width("C♯m7-"),
        "反転の始まりは表示幅で数える"
    );
    assert_eq!(total_width(&spans), 20);
}

/// 画面に収まらない行では `…` で切れる。**切れた向こう側の chord は反転が見えない**
/// だけで、桁も落ちないし panic もしない。
#[test]
fn a_chord_beyond_the_ellipsis_is_simply_not_highlighted() {
    let degrees = "I-V-VIm-IV-IIm-V-I-VIm";
    let last = degrees.rfind("VIm").expect("最後の chord があること");

    let spans = degrees_spans(degrees, 8, Some(last..degrees.len()), base_style());

    assert_eq!(total_width(&spans), 8);
    assert!(texts(&spans).concat().contains(ELLIPSIS), "{spans:?}");
    assert_eq!(reversed(&spans), "", "画面外なので反転する桁が無い");
}

/// 切れ目にまたがる chord は、**見えているところだけ**反転する。
#[test]
fn a_chord_cut_in_half_highlights_only_the_visible_part() {
    let degrees = "I-VIm-IV";
    let vim = degrees.find("VIm").expect("2 つめの chord があること");

    // `I-VI` まで（4 桁）＋ `…` で 5 桁。
    let spans = degrees_spans(degrees, 5, Some(vim..vim + 3), base_style());

    assert_eq!(total_width(&spans), 5);
    assert_eq!(reversed(&spans), "VI…", "見えている桁だけが反転する");
}

/// 幅 0 の pane（枠しか無い）でも落ちない。
#[test]
fn a_zero_width_column_draws_nothing() {
    let spans = degrees_spans("I-V", 0, Some(0..1), base_style());

    assert_eq!(texts(&spans).concat(), "");
}

/// 文字の途中で切れた範囲（写しが古いときに起きうる）は、**反転せずに素通し**する。
/// `&degrees[..n]` で桁を数えるので、ここで弾かないと panic する。
#[test]
fn a_range_off_the_character_boundary_falls_back_to_no_highlight() {
    let degrees = "C♯m7";

    // `♯` の 2 バイト目。
    let spans = degrees_spans(degrees, 10, Some(1..2), base_style());

    assert_eq!(texts(&spans), vec![fit_width(degrees, 10)]);
    assert_eq!(reversed(&spans), "");
}

/// 空の範囲も素通し（0 桁だけ反転しても見えない）。
#[test]
fn an_empty_range_falls_back_to_no_highlight() {
    let spans = degrees_spans("I-V", 10, Some(2..2), base_style());

    assert_eq!(spans.len(), 1);
    assert_eq!(reversed(&spans), "");
}
