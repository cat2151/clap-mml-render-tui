//! 共通PatchRoleをselectorのRole/Preset paneへ投影する。

use std::sync::Arc;

use cmrt_patches::{builtin_role_presets, normalize_user_role_presets, PatchRole, PatchRolePreset};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilterGroup {
    All,
    Role(PatchRole),
}

impl FilterGroup {
    pub(crate) const ALL: [Self; 7] = [
        Self::All,
        Self::Role(PatchRole::Bass),
        Self::Role(PatchRole::Chord),
        Self::Role(PatchRole::Lead),
        Self::Role(PatchRole::Drum),
        Self::Role(PatchRole::Triggered),
        Self::Role(PatchRole::Etc),
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::Role(PatchRole::Bass) => "Bass track",
            Self::Role(PatchRole::Chord) => "Chord track",
            Self::Role(PatchRole::Lead) => "Lead / melody",
            Self::Role(PatchRole::Drum) => "Drum tracks",
            Self::Role(PatchRole::Triggered) => "Triggered phrase",
            Self::Role(PatchRole::Etc) => "Etc / unknown",
        }
    }

    fn preset_prefix(self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::Role(PatchRole::Bass) => "Bass",
            Self::Role(PatchRole::Chord) => "Chord",
            Self::Role(PatchRole::Lead) => "Lead",
            Self::Role(PatchRole::Drum) => "Drum",
            Self::Role(PatchRole::Triggered) => "Triggered",
            Self::Role(PatchRole::Etc) => "Etc",
        }
    }

    pub(super) fn role(self) -> Option<PatchRole> {
        match self {
            Self::All => None,
            Self::Role(role) => Some(role),
        }
    }

    pub(super) fn user_destination(self) -> PatchRole {
        self.role().unwrap_or(PatchRole::Etc)
    }
}

#[derive(Clone)]
pub(crate) struct FilterPreset {
    pub(crate) label: String,
    pub(crate) pattern: Option<String>,
    pub(crate) is_user: bool,
    pub(crate) group: FilterGroup,
    pub(super) matches: Arc<[usize]>,
}

impl FilterPreset {
    pub(super) fn qualify_label(mut self) -> Self {
        self.label = format!("{} › {}", self.group.preset_prefix(), self.label);
        self
    }
}

pub(super) fn presets_for(
    group: FilterGroup,
    user_presets: &[(String, String)],
) -> Vec<FilterPreset> {
    let mut presets = vec![FilterPreset {
        label: "ALL".to_string(),
        pattern: None,
        is_user: false,
        group,
        matches: Arc::default(),
    }];
    let Some(role) = group.role() else {
        return presets;
    };
    presets.extend(
        builtin_role_presets()
            .iter()
            .filter(|preset| preset.role == role)
            .map(|preset| from_builtin(group, preset)),
    );
    presets.extend(
        user_presets
            .iter()
            .filter(|(key, _)| PatchRole::from_key(key) == role)
            .map(|(_, pattern)| FilterPreset {
                label: pattern.clone(),
                pattern: Some(pattern.clone()),
                is_user: true,
                group,
                matches: Arc::default(),
            }),
    );
    presets
}

fn from_builtin(group: FilterGroup, preset: &PatchRolePreset) -> FilterPreset {
    FilterPreset {
        label: preset.label.to_string(),
        pattern: Some(preset.pattern.to_string()),
        is_user: false,
        group,
        matches: Arc::default(),
    }
}

pub(super) fn normalize_user_presets(presets: Vec<(String, String)>) -> Vec<(String, String)> {
    normalize_user_role_presets(presets)
}

pub(super) fn patterns_for_role(role: PatchRole, user_presets: &[(String, String)]) -> Vec<String> {
    builtin_role_presets()
        .iter()
        .filter(|preset| preset.role == role)
        .map(|preset| preset.pattern.to_string())
        .chain(
            user_presets
                .iter()
                .filter(move |(key, _)| PatchRole::from_key(key) == role)
                .map(|(_, pattern)| pattern.clone()),
        )
        .collect()
}

#[cfg(test)]
pub(super) fn builtin_patterns() -> impl Iterator<Item = &'static str> {
    builtin_role_presets().iter().map(|preset| preset.pattern)
}
