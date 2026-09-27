//! 音色 selector と演奏設定の描画。持つ側は自分の画面の上へ、この順で重ねて描く。

mod auto_reverb;
mod notice;
mod patch_select;
mod play_settings;

pub use patch_select::{load_time_label, scroll_offset, PatchSelectDrawOptions};

use ratatui::{layout::Rect, Frame};

use crate::{DirectPatchSelect, PatchAuditionSelect, PatchSelect};

/// selector が開いていれば描く。
pub fn draw_patch_select(select: &PatchAuditionSelect<'_>, frame: &mut Frame<'_>) {
    if let Some(select) = select.select() {
        patch_select::draw(select, frame);
    }
}

/// [`PatchSelect`] を直接持つ host が、自分の枠の中の `area` へ描く。
pub fn draw_patch_select_in(
    select: &PatchSelect<'_>,
    frame: &mut Frame<'_>,
    area: Rect,
    options: &PatchSelectDrawOptions<'_>,
) {
    patch_select::draw_in(select, frame, area, options);
}

/// 演奏設定が開いていれば描く。音色選択の最中にも開ける手前のモーダルなので、
/// [`draw_patch_select`] より後に描くこと。
pub fn draw_play_settings(select: &PatchAuditionSelect<'_>, frame: &mut Frame<'_>) {
    if let Some(select) = select.play_settings_select() {
        play_settings::draw(select, frame);
    }
}

/// 入力欄なしで開いた selector を、開けていない理由・演奏設定まで含めてこの順で描く。
pub fn draw_direct_patch_select(select: &DirectPatchSelect<'_>, frame: &mut Frame<'_>) {
    let select = select.audition_select();
    if let Some(notice) = select.notice() {
        notice::draw(notice, frame);
    }
    draw_patch_select(select, frame);
    draw_play_settings(select, frame);
}
