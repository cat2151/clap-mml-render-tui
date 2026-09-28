//! 表示パスの階層を近い順に遡る、Role・Drum部位・labelの判定。

use super::{condition::Condition, PatchRole, PatchRoleInput, PatchRolePreset};

pub(super) struct CompiledRole {
    pub(super) role: PatchRole,
    pub(super) alternatives: Vec<Condition>,
}

impl CompiledRole {
    fn matches(&self, entry: PatchRoleInput<'_>) -> bool {
        self.alternatives
            .iter()
            .any(|condition| condition.matches(entry))
    }

    fn matches_text(&self, plugin: Option<&str>, text: &str) -> bool {
        self.alternatives
            .iter()
            .any(|condition| condition.matches_text(plugin, text))
    }
}

/// 楽器の根拠にしない表示パスの階層（小文字）。作者名が楽器語に当たるもの。
///
/// `\bsax`が作者名`Jeff Saxe`に当たり、その作者のdrumやclavまでsaxになるため。
const IGNORED_LEVELS: [&str; 1] = ["jeff saxe"];

/// 分類で見る階層。selector category、patch名、その上のフォルダを1つずつ遡る順。
///
/// 表示パスはフォルダと音色名を含むので、全体へ一度に当てると`Brass/Sax/...`のsaxが
/// cascadeで先のbrassに負ける。categoryはpatch名より先に置く。categoryを持つpluginの
/// patch名は`Leads/Synth Guitar`や`Basses/Hate`のように用途と違う語を含みやすい。
pub(super) fn levels<'a>(entry: PatchRoleInput<'a>) -> Vec<&'a str> {
    let mut levels: Vec<&str> = entry.selector_category.into_iter().collect();
    levels.extend(
        entry
            .normalized_display
            .rsplit('/')
            .filter(|level| !IGNORED_LEVELS.contains(level)),
    );
    levels
}

/// 階層で決まらなかったときに、表示名全体とcategoryへ当てる入力。無視する階層は除く。
pub(super) fn without_ignored_levels(normalized_display: &str) -> String {
    normalized_display
        .split('/')
        .filter(|level| !IGNORED_LEVELS.contains(level))
        .collect::<Vec<_>>()
        .join("/")
}

/// Roleと、それを決めた階層。階層で決まらなければ階層は`None`。
pub(super) fn classify_role<'a>(
    levels: &[&'a str],
    fallback: PatchRoleInput<'_>,
    cascade: &[CompiledRole],
) -> (PatchRole, Option<&'a str>) {
    let (triggered, roles) = cascade
        .split_first()
        .expect("classification cascade contains Triggered");
    debug_assert_eq!(triggered.role, PatchRole::Triggered);
    // 音色自身が鳴らすarpやsequenceは楽器の種類ではないので、どの階層に現れても最優先。
    if triggered.matches(fallback) {
        return (PatchRole::Triggered, None);
    }
    for level in levels {
        if let Some(compiled) = roles
            .iter()
            .find(|compiled| compiled.matches_text(fallback.plugin, level))
        {
            return (compiled.role, Some(level));
        }
    }
    // 複数termのユーザー規則は、階層やcategoryをまたいでAND一致できる。
    let role = roles
        .iter()
        .find(|compiled| compiled.matches(fallback))
        .map_or(PatchRole::Etc, |compiled| compiled.role);
    (role, None)
}

/// Roleを決めた階層があればその中だけで、無ければ表示名全体とcategoryで探す。
pub(super) fn builtin_label_within_role(
    decided_by: Option<&str>,
    fallback: PatchRoleInput<'_>,
    role: PatchRole,
    builtin_conditions: &[(&PatchRolePreset, Condition)],
) -> Option<&'static str> {
    builtin_conditions
        .iter()
        .find(|(preset, condition)| {
            preset.role == role
                && match decided_by {
                    Some(level) => condition.matches_text(fallback.plugin, level),
                    None => condition.matches(fallback),
                }
        })
        .map(|(preset, _)| preset.label)
}
