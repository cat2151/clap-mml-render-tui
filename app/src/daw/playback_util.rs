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
mod tests {
    use super::super::MEASURES;
    use super::{
        effective_measure_count, format_measure_list, loop_measure_summary_label,
        play_start_log_lines,
    };

    #[test]
    fn effective_measure_count_all_empty_returns_none() {
        let mmls = vec!["".to_string(); MEASURES];
        assert_eq!(effective_measure_count(&mmls), None);
    }

    #[test]
    fn effective_measure_count_skips_trailing_empty_measures() {
        let mut mmls = vec!["".to_string(); MEASURES];
        mmls[0] = "cccccccc".to_string();
        mmls[1] = "ffffffff".to_string();
        assert_eq!(effective_measure_count(&mmls), Some(2));
    }

    #[test]
    fn effective_measure_count_includes_internal_empty_measures() {
        let mut mmls = vec!["".to_string(); MEASURES];
        mmls[0] = "cde".to_string();
        mmls[2] = "fga".to_string();
        assert_eq!(effective_measure_count(&mmls), Some(3));
    }

    #[test]
    fn effective_measure_count_single_non_empty_measure() {
        let mut mmls = vec!["".to_string(); MEASURES];
        mmls[0] = "c".to_string();
        assert_eq!(effective_measure_count(&mmls), Some(1));
    }

    #[test]
    fn effective_measure_count_all_measures_non_empty() {
        let mmls: Vec<String> = (0..MEASURES).map(|i| format!("c{}", i)).collect();
        assert_eq!(effective_measure_count(&mmls), Some(MEASURES));
    }

    #[test]
    fn effective_measure_count_whitespace_only_treated_as_empty() {
        let mut mmls = vec!["".to_string(); MEASURES];
        mmls[0] = "cde".to_string();
        mmls[1] = "   ".to_string();
        assert_eq!(effective_measure_count(&mmls), Some(1));
    }

    #[test]
    fn format_measure_list_merges_consecutive_ranges() {
        assert_eq!(
            format_measure_list(&[1, 2, 3, 5, 7, 8]),
            Some("meas 1～3, meas 5, meas 7～8".to_string())
        );
    }

    #[test]
    fn loop_measure_summary_label_lists_loop_and_empty_ranges() {
        let mut mmls = vec![String::new(); MEASURES];
        mmls[0] = "c".to_string();
        mmls[2] = "g".to_string();

        assert_eq!(
            loop_measure_summary_label(&mmls),
            Some("loop meas : meas 1～3, empty meas : meas 2, meas 4～8".to_string())
        );
    }

    #[test]
    fn play_start_log_lines_describe_active_and_empty_measures() {
        let mut mmls = vec![String::new(); MEASURES];
        mmls[0] = "c".to_string();

        assert_eq!(
            play_start_log_lines(&mmls),
            vec![
                "meas1 : 内容があります".to_string(),
                "meas2 : empty".to_string(),
                "meas3 : empty".to_string(),
                "meas4 : empty".to_string(),
                "meas5 : empty".to_string(),
                "meas6 : empty".to_string(),
                "meas7 : empty".to_string(),
                "meas8 : empty".to_string(),
                "有効meas : meas 1".to_string(),
                "empty meas : meas 2～8".to_string(),
                "loop start meas : meas1".to_string(),
                "loop end meas : meas1".to_string(),
            ]
        );
    }
}
