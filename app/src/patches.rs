use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::config::Config;

const PATCH_DIR_PREFIXES: [&str; 2] = ["patches_factory", "patches_3rdparty"];

pub(crate) fn configured_patch_dirs(cfg: &Config) -> Vec<String> {
    cfg.patches_dirs
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|dir| !dir.trim().is_empty())
        .collect()
}

pub(crate) fn has_configured_patch_dirs(cfg: &Config) -> bool {
    !configured_patch_dirs(cfg).is_empty()
}

pub(crate) fn core_config_patch_root_dir(cfg: &Config) -> Option<String> {
    shared_patch_root_dir(&configured_patch_dirs(cfg))
}

fn normalize_patch_lookup_key(patch_name: &str) -> String {
    patch_name
        .trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_matches('/')
        .to_lowercase()
}

fn next_chunk(input: &str, start: usize) -> Option<(usize, bool)> {
    let mut chars = input[start..].char_indices();
    let (_, first) = chars.next()?;
    let is_digit = first.is_ascii_digit();
    let end = chars
        .find(|(_, ch)| ch.is_ascii_digit() != is_digit)
        .map(|(index, _)| start + index)
        .unwrap_or(input.len());
    Some((end, is_digit))
}

fn compare_natural_str(left: &str, right: &str) -> Ordering {
    let mut left_index = 0;
    let mut right_index = 0;

    while let (Some((left_end, left_is_digit)), Some((right_end, right_is_digit))) =
        (next_chunk(left, left_index), next_chunk(right, right_index))
    {
        let left_chunk = &left[left_index..left_end];
        let right_chunk = &right[right_index..right_end];

        let ordering = if left_is_digit && right_is_digit {
            let left_trimmed = left_chunk.trim_start_matches('0');
            let right_trimmed = right_chunk.trim_start_matches('0');
            let left_number = if left_trimmed.is_empty() {
                "0"
            } else {
                left_trimmed
            };
            let right_number = if right_trimmed.is_empty() {
                "0"
            } else {
                right_trimmed
            };

            left_number
                .len()
                .cmp(&right_number.len())
                .then_with(|| left_number.cmp(right_number))
                .then_with(|| left_chunk.len().cmp(&right_chunk.len()))
        } else {
            left_chunk.cmp(right_chunk)
        };

        if ordering != Ordering::Equal {
            return ordering;
        }

        left_index = left_end;
        right_index = right_end;
    }

    left[left_index..].cmp(&right[right_index..])
}

pub(crate) fn compare_patch_names_natural(left: &str, right: &str) -> Ordering {
    compare_natural_str(
        &normalize_patch_lookup_key(left),
        &normalize_patch_lookup_key(right),
    )
    .then_with(|| left.cmp(right))
}

pub(crate) fn resolve_display_patch_name(
    pairs: &[(String, String)],
    patch_name: &str,
) -> Option<String> {
    let key = normalize_patch_lookup_key(patch_name);
    if key.is_empty() {
        return None;
    }

    let mut candidates = vec![key.clone()];
    if !PATCH_DIR_PREFIXES
        .iter()
        .any(|prefix| key == *prefix || key.starts_with(&format!("{prefix}/")))
    {
        candidates.extend(
            PATCH_DIR_PREFIXES
                .iter()
                .map(|prefix| format!("{prefix}/{key}")),
        );
    }

    candidates.into_iter().find_map(|candidate| {
        pairs
            .iter()
            .find(|(_, lower)| lower == &candidate)
            .map(|(display, _)| display.clone())
    })
}

pub(crate) fn collect_patch_pairs(cfg: &Config) -> Result<Vec<(String, String)>> {
    let dirs = configured_patch_dirs(cfg);
    let Some(base_dir) = shared_patch_root_dir(&dirs) else {
        return collect_patch_pairs_with_optional_base(&dirs, None);
    };
    collect_patch_pairs_with_optional_base(&dirs, Some(base_dir.as_str()))
}

fn collect_patch_pairs_with_optional_base(
    dirs: &[String],
    base_dir: Option<&str>,
) -> Result<Vec<(String, String)>> {
    let mut pairs = Vec::new();
    for dir in dirs {
        let paths = cmrt_core::collect_patches(dir)?;
        pairs.extend(paths.into_iter().map(|path| {
            let display = match base_dir {
                Some(base_dir) => cmrt_core::to_relative(base_dir, &path),
                None => path.to_string_lossy().into_owned(),
            };
            let lower = display.to_lowercase();
            (display, lower)
        }));
    }
    pairs.sort_by(|(left_display, left_lower), (right_display, right_lower)| {
        compare_natural_str(left_lower, right_lower).then_with(|| left_display.cmp(right_display))
    });
    Ok(pairs)
}

fn shared_patch_root_dir(dirs: &[String]) -> Option<String> {
    let mut dir_paths = dirs.iter().map(PathBuf::from);
    let mut common = dir_paths.next()?;
    for dir in dir_paths {
        while !Path::new(&dir).starts_with(&common) {
            if !common.pop() {
                return None;
            }
        }
    }
    if common.as_os_str().is_empty() {
        return None;
    }
    Some(common.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn shared_patch_root_dir_returns_single_dir_as_is() {
        let dirs = vec!["/tmp/patches_factory".to_string()];

        let base = shared_patch_root_dir(&dirs);

        assert_eq!(base.as_deref(), Some("/tmp/patches_factory"));
    }

    #[test]
    fn shared_patch_root_dir_returns_common_parent_for_multiple_dirs() {
        let dirs = vec![
            "/tmp/surge-data/patches_factory".to_string(),
            "/tmp/surge-data/patches_3rdparty".to_string(),
        ];

        let base = shared_patch_root_dir(&dirs);

        assert_eq!(base.as_deref(), Some("/tmp/surge-data"));
    }

    #[test]
    fn shared_patch_root_dir_returns_none_when_only_empty_root_matches() {
        let dirs = vec![
            "patches_factory".to_string(),
            "patches_3rdparty".to_string(),
        ];

        let base = shared_patch_root_dir(&dirs);

        assert_eq!(base, None);
    }

    #[test]
    fn collect_patch_pairs_combines_factory_and_thirdparty_using_common_base() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("cmrt_collect_patch_pairs_{suffix}"));
        let factory = root.join("patches_factory");
        let thirdparty = root.join("patches_3rdparty");
        std::fs::create_dir_all(factory.join("Pads")).unwrap();
        std::fs::create_dir_all(thirdparty.join("Leads")).unwrap();
        std::fs::write(factory.join("Pads").join("Factory Pad.fxp"), b"dummy").unwrap();
        std::fs::write(thirdparty.join("Leads").join("Third Lead.fxp"), b"dummy").unwrap();

        let cfg = Config {
            plugin_path: String::new(),
            input_midi: String::new(),
            output_midi: String::new(),
            output_wav: String::new(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patches_dirs: Some(vec![
                factory.to_string_lossy().into_owned(),
                thirdparty.to_string_lossy().into_owned(),
            ]),
        };

        let pairs = collect_patch_pairs(&cfg).unwrap();

        assert!(pairs.contains(&(
            "patches_factory/Pads/Factory Pad.fxp".to_string(),
            "patches_factory/pads/factory pad.fxp".to_string()
        )));
        assert!(pairs.contains(&(
            "patches_3rdparty/Leads/Third Lead.fxp".to_string(),
            "patches_3rdparty/leads/third lead.fxp".to_string()
        )));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn collect_patch_pairs_sorts_display_names_naturally() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("cmrt_collect_patch_pairs_natural_{suffix}"));
        let factory = root.join("patches_factory");
        let pads = factory.join("Pads");
        std::fs::create_dir_all(&pads).unwrap();
        std::fs::write(pads.join("Pad 11.fxp"), b"dummy").unwrap();
        std::fs::write(pads.join("Pad 2.fxp"), b"dummy").unwrap();
        std::fs::write(pads.join("Pad 1.fxp"), b"dummy").unwrap();

        let cfg = Config {
            plugin_path: String::new(),
            input_midi: String::new(),
            output_midi: String::new(),
            output_wav: String::new(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patches_dirs: Some(vec![factory.to_string_lossy().into_owned()]),
        };

        let pairs = collect_patch_pairs(&cfg).unwrap();

        assert_eq!(
            pairs
                .into_iter()
                .map(|(display, _)| display)
                .collect::<Vec<_>>(),
            vec![
                "Pads/Pad 1.fxp".to_string(),
                "Pads/Pad 2.fxp".to_string(),
                "Pads/Pad 11.fxp".to_string(),
            ]
        );

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn compare_patch_names_natural_orders_numeric_suffixes() {
        let mut items = vec![
            "Pads/Pad 11.fxp".to_string(),
            "Pads/Pad 2.fxp".to_string(),
            "Pads/Pad 1.fxp".to_string(),
        ];
        items.sort_by(|left, right| compare_patch_names_natural(left, right));

        assert_eq!(
            items,
            vec![
                "Pads/Pad 1.fxp".to_string(),
                "Pads/Pad 2.fxp".to_string(),
                "Pads/Pad 11.fxp".to_string(),
            ]
        );
    }

    #[test]
    fn resolve_display_patch_name_adds_factory_prefix_when_missing() {
        let pairs = vec![
            (
                "patches_factory/Pads/Factory Pad.fxp".to_string(),
                "patches_factory/pads/factory pad.fxp".to_string(),
            ),
            (
                "patches_3rdparty/Leads/Third Lead.fxp".to_string(),
                "patches_3rdparty/leads/third lead.fxp".to_string(),
            ),
        ];

        let resolved = resolve_display_patch_name(&pairs, "Pads/Factory Pad.fxp");

        assert_eq!(
            resolved.as_deref(),
            Some("patches_factory/Pads/Factory Pad.fxp")
        );
    }

    #[test]
    fn resolve_display_patch_name_prefers_existing_prefixed_name() {
        let pairs = vec![(
            "patches_3rdparty/Leads/Third Lead.fxp".to_string(),
            "patches_3rdparty/leads/third lead.fxp".to_string(),
        )];

        let resolved = resolve_display_patch_name(&pairs, "patches_3rdparty/Leads/Third Lead.fxp");

        assert_eq!(
            resolved.as_deref(),
            Some("patches_3rdparty/Leads/Third Lead.fxp")
        );
    }
}
