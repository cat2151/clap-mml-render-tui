//! README.ja.md の図と実装を突き合わせる。
//!
//! 図は手で書くので、下段 1 行のキー要約を削っても README だけ古いまま残る。
//! **`README.md` は `README.ja.md` からの生成物なので読まない**（`AGENTS.md`）。

use ratatui::text::Span;

/// リポジトリルートの `README.ja.md`。crate の位置から辿る（cwd に依存させない）。
const README: &str = include_str!("../../../../README.ja.md");

/// 図の枠の内側の桁数。画面の実装（`layout_for` の `status.width`）と同じ 78 桁。
const FIGURE_INNER_WIDTH: usize = 78;

/// chord chart 画面の図（```で囲まれた、`┌ Chord Chart` で始まる塊）。
fn figure_lines() -> Vec<&'static str> {
    let start = README
        .lines()
        .position(|line| line.starts_with("┌ Chord Chart"))
        .expect("README.ja.md に chord chart 画面の図が無い");
    README
        .lines()
        .skip(start)
        .take_while(|line| !line.starts_with("```"))
        .collect()
}

/// 図の各行が 80 桁ちょうどか。全角を 1 桁で数えると 2 桁はみ出す（実際に出た）。
#[test]
fn every_line_of_the_readme_figure_is_eighty_columns_wide() {
    for line in figure_lines() {
        assert_eq!(
            Span::raw(line).width(),
            FIGURE_INNER_WIDTH + 2,
            "図の行の幅が違う: {line:?}"
        );
    }
}

/// 図の下段 1 行が、実装の [`KEYBIND_TEXT`](crate::ui::help) そのままか。
#[test]
fn the_readme_figure_shows_the_real_bottom_line() {
    let lines = figure_lines();
    let status = lines[lines.len() - 2];
    let inner = status
        .strip_prefix('│')
        .and_then(|line| line.strip_suffix('│'))
        .unwrap_or_else(|| panic!("図の下段が枠で囲まれていない: {status:?}"));

    assert_eq!(inner.trim_end(), crate::ui::help::KEYBIND_TEXT);
    assert_eq!(Span::raw(inner).width(), FIGURE_INNER_WIDTH);
}

/// 「音は鳴りません」という説明が残っていないか。
///
/// preview を足した Stage 4 でいちばん古びる 1 文。綴りで名指ししておかないと、
/// 図とキー表だけ直して本文が取り残される。
#[test]
fn the_readme_no_longer_says_the_screen_is_silent() {
    assert!(
        !README.contains("音は鳴りません"),
        "README が古いままになっている"
    );
}
