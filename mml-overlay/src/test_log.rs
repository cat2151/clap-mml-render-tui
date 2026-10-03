//! テストから [`super::log_line`] の出力を読む。
//!
//! sink は 1 度しか注入できず、テストは並列に走る。注入をここ 1 か所に寄せ、どのテストも
//! 同じ記録を読む。他のテストの行も混ざるので、読む側は自分の行だけを選び出すこと。

use std::sync::Mutex;

static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn record(line: &str) {
    LINES.lock().unwrap().push(line.to_string());
}

/// 記録を始める。何度呼んでもよい。
pub(crate) fn install() {
    super::set_log_sink(record);
}

/// これまでに記録した行のうち、`needles` をすべて含むもの。
pub(crate) fn lines_containing(needles: &[&str]) -> Vec<String> {
    LINES
        .lock()
        .unwrap()
        .iter()
        .filter(|line| needles.iter().all(|needle| line.contains(needle)))
        .cloned()
        .collect()
}
