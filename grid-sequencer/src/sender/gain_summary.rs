/// 0 dB でない行だけを `row2:+6dB` のように並べる。全部 0 dB なら `none`。
pub(super) fn describe_boosted(gains_db: &[f32]) -> String {
    let boosted = gains_db
        .iter()
        .enumerate()
        .filter(|(_, gain)| **gain != 0.0)
        .map(|(index, gain)| format!("row{}:{gain:+}dB", index + 1))
        .collect::<Vec<_>>();
    if boosted.is_empty() {
        "none".to_string()
    } else {
        boosted.join(",")
    }
}
