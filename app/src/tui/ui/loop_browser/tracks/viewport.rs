pub(super) fn keep_visible(cursor: usize, visible: usize, scroll: &mut usize) {
    if cursor < *scroll {
        *scroll = cursor;
    } else if cursor >= *scroll + visible {
        *scroll = cursor + 1 - visible;
    }
}

pub(super) fn fit(text: &str, width: usize) -> String {
    let mut output = text.chars().take(width).collect::<String>();
    let count = output.chars().count();
    output.extend(std::iter::repeat_n(' ', width.saturating_sub(count)));
    output
}
