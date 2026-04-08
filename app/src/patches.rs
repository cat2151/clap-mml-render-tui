use std::{
    collections::HashMap,
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

    let resolved_pairs = pairs
        .iter()
        .map(|(display, _)| (normalize_patch_lookup_key(display), display))
        .collect::<HashMap<_, _>>();

    candidates.into_iter().find_map(|candidate| {
        resolved_pairs
            .get(&candidate)
            .map(|display| (*display).clone())
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
