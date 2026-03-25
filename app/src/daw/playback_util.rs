/// 末尾の空小節を除いた有効な小節数を計算する。
///
/// すべての小節が空の場合は `None` を返す。
/// これにより meas3-8 が空のときは meas1-2 だけをループする（issue #68）。
pub(super) fn effective_measure_count(mmls: &[String]) -> Option<usize> {
    mmls.iter()
        .rposition(|m| !m.trim().is_empty())
        .map(|idx| idx + 1)
}

fn measure_indices_matching(mmls: &[String], is_match: impl Fn(&str) -> bool) -> Vec<usize> {
    mmls.iter()
        .enumerate()
        .filter_map(|(idx, mml)| is_match(mml.trim()).then_some(idx + 1))
        .collect()
}

pub(super) fn non_empty_measure_indices(mmls: &[String]) -> Vec<usize> {
    measure_indices_matching(mmls, |mml| !mml.is_empty())
}

pub(super) fn empty_measure_indices(mmls: &[String]) -> Vec<usize> {
    measure_indices_matching(mmls, str::is_empty)
}

pub(super) fn format_measure_list(indices: &[usize]) -> Option<String> {
    if indices.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let mut start = indices[0];
    let mut prev = indices[0];

    for &index in &indices[1..] {
        if index == prev + 1 {
            prev = index;
            continue;
        }

        if start == prev {
            parts.push(format!("meas {start}"));
        } else {
            parts.push(format!("meas {start}～{prev}"));
        }
        start = index;
        prev = index;
    }

    if start == prev {
        parts.push(format!("meas {start}"));
    } else {
        parts.push(format!("meas {start}～{prev}"));
    }

    Some(parts.join(", "))
}

pub(super) fn loop_measure_summary_label(mmls: &[String]) -> Option<String> {
    let effective_count = effective_measure_count(mmls)?;
    let loop_measures: Vec<usize> = (1..=effective_count).collect();
    let loop_label = format_measure_list(&loop_measures)?;
    let empty_label =
        format_measure_list(&empty_measure_indices(mmls)).unwrap_or_else(|| "none".to_string());
    Some(format!(
        "loop meas : {loop_label}, empty meas : {empty_label}"
    ))
}

pub(super) fn play_start_log_lines(mmls: &[String]) -> Vec<String> {
    let Some(effective_count) = effective_measure_count(mmls) else {
        return Vec::new();
    };

    let active_measures = non_empty_measure_indices(mmls);
    let empty_measures = empty_measure_indices(mmls);
    let mut lines: Vec<String> = mmls
        .iter()
        .enumerate()
        .map(|(idx, mml)| {
            if mml.trim().is_empty() {
                format!("meas{} : empty", idx + 1)
            } else {
                format!("meas{} : 内容があります", idx + 1)
            }
        })
        .collect();

    lines.push(format!(
        "有効meas : {}",
        format_measure_list(&active_measures).unwrap_or_else(|| "none".to_string())
    ));
    lines.push(format!(
        "empty meas : {}",
        format_measure_list(&empty_measures).unwrap_or_else(|| "none".to_string())
    ));
    lines.push("loop start meas : meas1".to_string());
    lines.push(format!("loop end meas : meas{effective_count}"));
    lines
}

#[cfg(test)]
#[path = "../tests/daw/playback_util.rs"]
mod tests;
