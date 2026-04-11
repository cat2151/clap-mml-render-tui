mod guide;
mod notepad_history;
mod patch_phrase;
mod patch_select;

fn selection_status_text(cursor: usize, total: usize) -> String {
    if total == 0 {
        "現在 0/0".to_string()
    } else {
        let current = cursor.min(total - 1) + 1;
        format!("現在 {current}行目 / 全{total}行 ({current}/{total})")
    }
}

pub(super) use guide::draw_notepad_history_guide;
pub(super) use notepad_history::draw_notepad_history;
pub(super) use patch_phrase::draw_patch_phrase;
pub(super) use patch_select::draw_patch_select;
