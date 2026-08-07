use ratatui::{
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    memory, status::base_style, theme::MONOKAI_CYAN, ui::centered_text_block_rect,
};

const TITLE: &str = " Grid Sequencer ヘルプ(Keybinds)  Esc/q/?:close ";

/// 画面下部に常に出しておく1行のキーバインド要約。
pub(super) const KEYBIND_TEXT: &str =
    " mouse:edit u:undo a:A/H x:clear c:chord r/R:random t:tracks Ctrl+G:screen q:quit";

pub(super) fn draw_overlay(f: &mut Frame<'_>, track_count: usize) {
    let lines = help_lines(track_count);
    let area = centered_text_block_rect(f.area(), TITLE, &lines);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(TITLE)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn help_lines(track_count: usize) -> Vec<Line<'static>> {
    // メモリ行は先頭に置く。端末が低いと centered_text_block_rect が下を切り落とすため。
    let mut lines = memory::overlay_lines();
    lines.extend(vec![
        Line::from(format!(
            "{track_count}行 x 16ステップ を常時ループ再生します。"
        )),
        Line::from("BPM130、1ステップ = 115ms(16分音符)、16ステップ = 1.85秒で1周。"),
        Line::from("  Ctrl+G   画面切替メニュー"),
        Line::from("  NOTE     # = Attack  - = Tie  . = Rest"),
        Line::from("  mouse    左drag:長いnote／row横断でarpeggio、左click:1step note/消去"),
        Line::from(
            "           右drag:note消去、PATCH click/wheel:選択/random、NOTE wheel:音高/転回",
        ),
        Line::from("  u        直前の編集をundo（mouse dragは通過cell数によらず1操作）"),
        Line::from("  a        AUTO/HOLD切替（AUTOはコード進行1周ごとに全行を再抽選）"),
        Line::from("           HOLD: 手編集した譜面を保持し、コード変更時は発音音高だけ更新"),
        Line::from("  x        全note消去（mouse編集とxはHOLDへ移行）"),
        Line::from("  c        chord mode の on/off"),
        Line::from("           instance 1は全音符の和音(AUTO時のみ+6dB)。instance 2は4 voiceを"),
        Line::from("           独立patternで鳴らします（同時でなくても可、patchは4行で共有）。"),
        Line::from("           高音が上、rootが下。NOTE wheelで最下音を1 chord noteずつ転回。"),
        Line::from("           mono patchも使用可。音が重なる場合の優先動作はplugin依存です。"),
        Line::from("           行1の音色は config.toml の chord_patch_categories から。"),
        Line::from("           on/off は t キーやアプリ終了をまたいで保存されます。"),
        Line::from("  r        grid を丸ごとランダム設定(patch / note / pattern)"),
        Line::from("  R        patch を据え置き、note / patternだけランダム設定"),
        Line::from("           (音色ロードが無いので再生が途切れない)"),
        Line::from("  t        track数を 1/2/4/8/16 で切替してアプリを再起動"),
        Line::from("  b        シングルバッファリングの on/off"),
        Line::from("  ?        このヘルプ"),
        Line::from("  q        終了"),
        Line::from(""),
        Line::from(format!(
            "{track_count} instancesは realtime play server の CLAP instance 0〜{} に対応し、",
            track_count - 1
        )),
        Line::from(format!(
            "instanceごとに別の音色です。r は{track_count} instanceぶんの音色ロードを"
        )),
        Line::from("やり直すため、その間だけ再生が止まります。"),
        Line::from(""),
        Line::from("準備中のlaneは同じinstance単位で色が落ちます。暗いグレー = 未構築、"),
        Line::from("グレー = instance のみ構築済み、通常色 = 音色ロードまで完了。"),
        Line::from(""),
        Line::from("シングルバッファリング(b)は裏での読み込みをやめ、進行を1周"),
        Line::from("鳴らしきる→音色ロード→先頭から再開、を繰り返します。ロード中は"),
        Line::from("無音になる代わりに、演奏中のブツ切れが無くなります。出力バッファを"),
        Line::from("上限まで厚くしてもフレームドロップが10秒続いたときは、自動で"),
        Line::from("こちらへ切り替わります(ステータス行が single / sb になります)。"),
    ]);
    lines
}
