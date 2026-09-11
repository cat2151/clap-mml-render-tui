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

/// キーの役目が変わったところを、README のキー表の**行ごと**名指しで固定する。
///
/// `h` `l` は pane 移動から行内の chord 移動へ移り、pane 移動は `Tab` になった。
/// キーの綴り（`h` `l`）は両方の役目に出てくるので、**綴りの有無を見ても気づけない**。
/// 動作の側を名指しする。
///
/// なお help overlay 側の集合との自動突き合わせはしていない。README は
/// `` `Alt+↑` `Alt+↓` ``、実装は `Alt+↑/↓` のように**同じキーを別の綴りで書く**ので、
/// 正規化を通しても等値にならず、例外表を書くほうが壊れやすくなる。
#[test]
fn the_readme_key_table_gives_h_and_l_their_new_job() {
    assert!(
        README.contains("| `Tab` | 共通 | pane移動"),
        "README のキー表に `Tab` の pane 移動が無い"
    );
    assert!(
        README.contains("| `h` `l` `←` `→` | 共通 | 行内のchord移動"),
        "README のキー表に `h` `l` の chord 移動が無い"
    );
    // 古い役目が 1 行でも残っていると、押しても pane が動かないキーを教える README になる。
    assert!(
        !README.contains("pane移動（`h`"),
        "README がまだ `h` `l` を pane 移動として教えている"
    );
}
