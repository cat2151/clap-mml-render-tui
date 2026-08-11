use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use cmrt_tui_core::{memory, status::base_style, theme::MONOKAI_CYAN, ui::centered_rect_with_size};

/// `cmrt_tui_core::buffer_test::help_overlay_bounds` が枠を探す目印でもあるので、
/// 「ヘルプ(Keybinds)」の並びは他画面と揃えたまま変えない。
const TITLE: &str = " Grid Sequencer ヘルプ(Keybinds)  Esc/q/?:close ";

/// 画面下部に常に出しておく1行のキーバインド要約。
pub(super) const KEYBIND_TEXT: &str =
    " mouse:edit u:undo a:A/H x:clear c:chord r/R:random t:tracks Ctrl+G:screen q:quit";

/// 枠線が食う幅・高さ。
const BORDER_SIZE: u16 = 2;
/// 左右 pane の間に空ける余白。
const PANE_GAP: u16 = 2;

/// ヘルプは2 pane。左はキーバインドだけ、右は非自明な挙動の説明だけを置く。
/// 「何のキーか」と「何が起きるか」を混ぜると、どちらも探しにくくなる。
pub(super) fn draw_overlay(f: &mut Frame<'_>, track_count: usize) {
    let keybinds = keybind_lines();
    let features = feature_lines(track_count);
    let area = overlay_rect(f.area(), &keybinds, &features);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(TITLE)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(pane_width(&keybinds)),
            Constraint::Length(PANE_GAP),
            Constraint::Min(0),
        ])
        .split(inner);
    f.render_widget(Paragraph::new(keybinds).style(base_style()), panes[0]);
    // 端末が狭いときは右 pane が削られる。折り返して読めるようにしておく。
    f.render_widget(
        Paragraph::new(features)
            .style(base_style())
            .wrap(Wrap { trim: false }),
        panes[2],
    );
}

/// 2 pane を並べた大きさの矩形。タイトルが本文より長い場合はタイトルに合わせる。
fn overlay_rect(area: Rect, keybinds: &[Line<'_>], features: &[Line<'_>]) -> Rect {
    let content_width = pane_width(keybinds)
        .saturating_add(PANE_GAP)
        .saturating_add(pane_width(features));
    let width = content_width
        .max(Line::from(TITLE).width() as u16)
        .saturating_add(BORDER_SIZE);
    let height = (keybinds.len().max(features.len()) as u16).saturating_add(BORDER_SIZE);
    centered_rect_with_size(width, height, area)
}

fn pane_width(lines: &[Line<'_>]) -> u16 {
    u16::try_from(lines.iter().map(Line::width).max().unwrap_or(0)).unwrap_or(u16::MAX)
}

/// 左 pane。キーと、その1行の意味だけ。全角=2セルで幅38に収める。
fn keybind_lines() -> Vec<Line<'static>> {
    [
        " Ctrl+G  画面切替メニュー",
        " 左click 1step の note / 消去",
        " 左drag  長い note、row横断で arpeggio",
        " 右drag  note 消去",
        " wheel   PATCH欄:音色を送る(↓次 ↑前)",
        "         NOTE欄:音高 / 転回(↑上げる)",
        "         grid:フレーズを送る(↓次 ↑前)",
        " u       直前の編集を undo",
        " a       AUTO / HOLD 切替",
        " x       全 note 消去",
        " c       chord mode on/off",
        " r       全ランダム(patch 含む)",
        " R       note/pattern だけランダム",
        " t       track数 1/2/3/4/8/16 切替",
        " b       シングルバッファリング切替",
        " ?       このヘルプ",
        " q       終了",
    ]
    .into_iter()
    .map(Line::from)
    .collect()
}

/// 右 pane。キーを見ても分からない挙動だけに絞る。
///
/// 90x24 の端末でも枠内へ収める前提で、幅48セル・18行に切り詰めてある。
/// 足すときは同じだけ削ること（溢れると右 pane が折り返して下が切れる）。
fn feature_lines(track_count: usize) -> Vec<Line<'static>> {
    // メモリ行は先頭に置く。端末が低いと overlay_rect が下を切り落とすため。
    let mut lines = memory::overlay_lines();
    lines.extend(
        [
            format!("{track_count}行 x 16step(115ms/16分, BPM130)を1.85秒で1周"),
            "NOTE欄  # = Attack   - = Tie   . = Rest".to_string(),
            "lane色  暗=未構築  灰=instanceのみ  通常=ロード済".to_string(),
            "chord mode(c) 行1=和音 行2=bass 行3=4 voice。".to_string(),
            "  転回とoctaveは全自動(auto voicing)で、top note".to_string(),
            "  の跳躍を最小化し、進行の引き直しもまたいで繋ぎ".to_string(),
            "  ます。on/off は保存されます。".to_string(),
            "grid の wheel↓ で次のフレーズ(chord mode中)".to_string(),
            "  4 voice行 Up/Down/UpDown/DownUp/UpDownHold/".to_string(),
            "    Converge/Diverge/Octave/Random".to_string(),
            "  bass行 Whole/8th/8th+Oct/#-##/#-##+Oct/".to_string(),
            "    ####+Oct  +Oct は八分ごとに octave 上と往復".to_string(),
            "    Whole は回した瞬間から鳴り出します。".to_string(),
            "AUTO / HOLD(a) AUTO は1周ごとに全行を再抽選。".to_string(),
            "  HOLD は手編集した譜面を保持し、コード変更時は".to_string(),
            "  発音音高だけ更新。mouse編集/x/生成で HOLD へ。".to_string(),
            "シングルバッファリング(b) 1周鳴らしきる→音色".to_string(),
            "  ロード→先頭から再開。ロード中は無音になる代わ".to_string(),
            "  りに演奏のブツ切れが無くなります(自動切替あり)".to_string(),
        ]
        .into_iter()
        .map(Line::from),
    );
    lines
}
