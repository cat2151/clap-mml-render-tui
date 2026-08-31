/// 等倍でない行だけを `row2:mute` / `row3:2.00x` のように並べる。
pub(super) fn describe_adjusted(gains: &[f32]) -> String {
    let adjusted = gains
        .iter()
        .enumerate()
        .filter(|(_, gain)| **gain != 1.0)
        .map(|(index, gain)| {
            let value = if *gain == 0.0 {
                "mute".to_string()
            } else {
                format!("{gain:.2}x")
            };
            format!("row{}:{value}", index + 1)
        })
        .collect::<Vec<_>>();
    if adjusted.is_empty() {
        "none".to_string()
    } else {
        adjusted.join(",")
    }
}
