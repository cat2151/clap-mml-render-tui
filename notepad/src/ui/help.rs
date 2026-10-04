use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::theme::{MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_YELLOW};

use super::{base_style, Mode};

const HELP_TITLE: &str = " ヘルプ (Keybinds) ";

pub(super) fn draw_help(f: &mut Frame, mode: Mode) {
    let help_lines = match mode {
        Mode::PatchSelect => vec![
            section_title("音色選択モード"),
            Line::from("  ?                   : ヘルプ (このページ)"),
            Line::from("  /                   : Regex 絞り込み (Role・Preset と AND)"),
            Line::from("  h / l ・ ← / →      : ペイン移動 (Role / Preset / 音色)"),
            Line::from("  j / k ・ ↑ / ↓      : 移動して再生"),
            Line::from("  PgUp/PgDn・Home/End : 10行 / 先頭・末尾へ移動して再生"),
            Line::from("  r                   : ランダムな音色へ移動して再生"),
            Line::from("  Space               : 現在行を再生"),
            Line::from("  Enter               : 音色決定"),
            Line::from("  a                   : Regex を Preset に追加"),
            Line::from("  f                   : 現在音色とMMLをFavorites追加"),
            Line::from("  m                   : plugin solo/mute"),
            Line::from("  e / E               : auto reverb on/off / ルール編集"),
            Line::from("  n / p / t           : notepad history / patch history / 音色選択"),
            escape_hint(),
        ],
        Mode::NotepadHistory => vec![
            section_title("notepad history 画面"),
            Line::from("  ?                 : ヘルプ (このページ)"),
            Line::from("  /                 : MML 絞り込み開始"),
            Line::from("  / の後に文字入力 : フィルタ (Space=AND条件)"),
            Line::from("  Enter             : 絞り込み入力を確定して操作に戻る"),
            Line::from("  n / p / t         : notepad history / patch history / 音色選択"),
            Line::from("  h / l ・ ← / →    : ペイン切替"),
            Line::from("  j / k ・ ↑ / ↓    : 上下移動して再生"),
            Line::from("  PageUp / PageDown : 1画面移動して再生"),
            Line::from("  Enter             : 現在行へ確定"),
            Line::from("  f                 : History行をお気に入りに追加"),
            Line::from("  dd                : Favorites行を削除してHistory先頭へ移動"),
            escape_hint(),
        ],
        Mode::PatchPhrase => vec![
            section_title("patch phrase 画面"),
            Line::from("  ?                 : ヘルプ (このページ)"),
            Line::from("  /                 : MML 絞り込み開始"),
            Line::from("  / の後に文字入力 : フィルタ (Space=AND条件)"),
            Line::from("  Enter             : 絞り込み入力を確定して操作に戻る"),
            Line::from("  n / p / t         : notepad history / patch history / 音色選択"),
            Line::from("  j / k ・ ↑ / ↓    : 上下移動して再生"),
            Line::from("  PageUp / PageDown : 1画面移動して再生"),
            Line::from("  h / l ・ ← / →    : ペイン切替して再生"),
            Line::from("  Space             : 現在行を再生"),
            Line::from("  Enter             : 現在行の上に挿入"),
            Line::from("  i                 : History行を編集"),
            Line::from("  f                 : 現在行をお気に入りに追加"),
            escape_hint(),
        ],
        Mode::EffectChain => vec![
            section_title("EFFECT CHAIN overlay (x)"),
            Line::from("  ?        : ヘルプ (このページ)"),
            Line::from("  j/k, ↓/↑ : 段の移動"),
            Line::from("  PageUp/PageDown : 段を10移動、Home/End : 先頭/末尾"),
            Line::from(
                "  a        : preset 一覧から chain 末尾へ追加（category/kind/list 3 pane、Enter、ESC で戻る）",
            ),
            Line::from(
                "  r        : 現在の段を preset 一覧から差し替え（add と同じ 3 pane、Enter で差し替え、ESC で戻る）",
            ),
            Line::from("  dd       : 現在の段を削除して preview"),
            Line::from("  b        : 現在の段の bypass を切り替えて preview"),
            Line::from("  Alt+↑/↓  : 現在の段を上下に並べ替えて preview"),
            Line::from("  Space    : 編集中 chain（bypass 反映）で現在行を preview"),
            Line::from("  Enter    : 行へ書き戻して閉じ、その行を再生"),
            Line::from("  ESC      : 破棄して閉じる"),
            escape_hint(),
        ],
        Mode::EffectChainAdd => vec![
            section_title("EFFECT CHAIN add / replace overlay (x → a / r)"),
            Line::from("  ?        : ヘルプ (このページ)"),
            Line::from("  h/l, ←/→ : category pane / kind pane / list pane"),
            Line::from("  j/k, ↓/↑ : focus 中の pane を移動して preview"),
            Line::from("  PageUp/PageDown/Home/End : focus 中の pane を大移動して preview"),
            Line::from("  /        : list を絞り込み（Enter=確定、ESC=編集前へ戻す）"),
            Line::from("  r        : list のランダムな候補へ移動して preview"),
            Line::from(
                "  Space    : 編集中 chain + list カーソルの preset を末尾に足して（r は差し替えて）preview（移動時も自動）",
            ),
            Line::from(
                "  b        : list カーソルの候補の段だけ bypass して preview（chain の他の段は効かせる。効き具合の比較用）",
            ),
            Line::from(
                "  Enter    : list カーソルの preset を chain 末尾へ追加して戻る（r で開いたときは現在の段と差し替え）",
            ),
            Line::from("  ESC      : 追加・差し替えせず chain 一覧へ戻る"),
            escape_hint(),
        ],
        _ => vec![
            section_title("NORMAL モード"),
            Line::from("  j / ↓       : 下へ移動して再生"),
            Line::from("  k / ↑       : 上へ移動して再生"),
            Line::from("  Alt+↑/↓     : 現在行を上 / 下へ並べ替え"),
            Line::from("  PageDown    : 1画面下へ移動して再生"),
            Line::from("  PageUp      : 1画面上へ移動して再生"),
            Line::from("  Home        : 先頭行へ移動して再生"),
            Line::from("  M           : 中央行へ移動して再生"),
            Line::from("  L           : 末尾行へ移動して再生"),
            Line::from("  Enter/Space : 再生"),
            Line::from("  i           : INSERT モード"),
            Line::from("  o / O       : 下 / 上に挿入 → INSERT"),
            Line::from("  dd / Del : 削除（ヤンク）  p / P : 下貼付 / 上貼付"),
            Line::from("  g           : generate を上に挿入して再生"),
            Line::from("  r           : ランダム音色を挿入/置換して再生"),
            Line::from("  t           : 音色選択"),
            Line::from("  x           : effect chain"),
            Line::from("  Shift+H     : patch history"),
            Line::from("  f           : patch phrase 画面"),
            Line::from("  e           : config.toml 編集 → 再起動"),
            Line::from("  w           : DAW モード"),
            Line::from("  Ctrl+G      : 画面切替"),
            Line::from("  K / ?       : ヘルプ (このページ)"),
            Line::from("  q           : 終了"),
            Line::from(""),
            section_title("INSERT モード"),
            Line::from("  ESC   : 確定 → NORMAL (再生)"),
            Line::from("  Enter : 確定 → 次行挿入 → INSERT 継続"),
            Line::from("  Ctrl+C: コピー"),
            Line::from("  Ctrl+X: カット"),
            Line::from("  Ctrl+V: ペースト"),
            escape_hint(),
        ],
    };

    // メモリ行は先頭に置く。ヘルプが端末より長いと centered_text_block_rect が
    // 下を切り落とすため、末尾に置くと見えなくなる。
    let help_lines = [cmrt_tui_core::memory::overlay_lines(), help_lines].concat();

    let area = cmrt_tui_core::ui::centered_text_block_rect(f.area(), HELP_TITLE, &help_lines);
    f.render_widget(Clear, area);

    f.render_widget(
        Paragraph::new(help_lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(HELP_TITLE)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn section_title(text: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        text,
        base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
    ))
}

fn escape_hint() -> Line<'static> {
    Line::from(Span::styled(
        "  [ESC] でキャンセル",
        base_style().fg(MONOKAI_GRAY),
    ))
}
