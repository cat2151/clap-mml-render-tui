//! 行の用途（[`PatchRole`]）と、この画面が持つ設定・voicing 判定との橋渡し。
//!
//! 「どのカテゴリから引くか」「poly を要求するか」といったplugin固有の判断は
//! play-server shared crateが返すmetadataにあり、ここはcontextから材料を渡すだけ。

use cmrt_patches::{candidates_for_role, PatchRole, RoleFilter, RoleFilterLookup, VoicingLookup};
use cmrt_realtime_play::PatchVoicing;
use cmrt_rhythm::DrumRole;
use cmrt_tui_core::patch_plugins::{PatchPlugins, PatchRoles};

use crate::{GridSequencerContext, GridVoicingLookup, ARPEGGIO_ROW, BASS_ROW, CHORD_ROW};

/// この画面の voicing 判定を、patch catalog 側の trait へ橋渡しするアダプタ。
pub(crate) struct PolyLookup<'a>(&'a dyn GridVoicingLookup);

impl VoicingLookup for PolyLookup<'_> {
    fn is_poly(&self, patch: &str) -> bool {
        self.0.cached_voicing(patch) == Some(PatchVoicing::Poly)
    }
}

/// 用途 1 つぶんの候補判定の材料を、カタログのプラグインごとに持つ表。
///
/// キーワードは小文字化して持つ必要があり、percussion では他3役ぶんの合併にもなる。
/// どちらも借用のままでは持てないので、この型が受け皿になる。
///
/// **プラグインごとに 1 組必要。** Surge のカテゴリを Dexed の cartridge へ当てると
/// 候補が全滅する（cartridge にカテゴリ階層が無い）。patch ごとに組み直すと patch 数ぶん
/// 確保が走るので、プラグイン数ぶんだけ先に組んでおき、走査中は添字で引く。
pub(crate) struct GridRoleFilters<'a> {
    role: PatchRole,
    plugins: &'a PatchPlugins,
    /// プラグインごとの (カテゴリ, 小文字化済みキーワード)。`plugins` と同じ並び。
    per_plugin: Vec<(&'a [String], Vec<String>)>,
}

impl RoleFilterLookup for GridRoleFilters<'_> {
    fn filter_for(&self, display: &str) -> RoleFilter<'_> {
        let index = self
            .plugins
            .index_for_patch(display)
            .expect("catalog内のpatchは所有pluginへroutingできること");
        let (categories, keywords) = &self.per_plugin[index];
        RoleFilter::with_keywords(self.role, categories, keywords)
    }
}

/// drum の役割に対応する patch の用途。
pub(crate) fn drum_patch_role(role: DrumRole) -> PatchRole {
    match role {
        DrumRole::Kick => PatchRole::Kick,
        DrumRole::Snare => PatchRole::Snare,
        DrumRole::HiHat => PatchRole::HiHat,
        DrumRole::Percussion => PatchRole::Percussion,
    }
}

/// 行（instance）の用途。chord mode の on/off と、その行の drum 割り当てで決まる。
///
/// drum 行は chord mode の on/off に関わらず用途が決まっている。chord mode 中は
/// 先頭 3 行の用途も決まっていて、それ以外は [`PatchRole::Free`]（＝和音向きの
/// 音色を避ける）。chord mode off なら drum 以外の全行が Free。
///
/// PATCH 欄の wheel と、画面を起動せずに候補を数える診断
/// （[`GridSequencerContext::role_candidates`]）が同じ規則を通るよう、ここ1か所に置く。
pub fn row_patch_role(instance: usize, chord_on: bool, drum: Option<DrumRole>) -> PatchRole {
    match drum {
        Some(drum) => drum_patch_role(drum),
        None => match instance {
            CHORD_ROW if chord_on => PatchRole::Chord,
            BASS_ROW if chord_on => PatchRole::Bass,
            ARPEGGIO_ROW if chord_on => PatchRole::Arpeggio,
            _ => PatchRole::Free,
        },
    }
}

impl GridSequencerContext<'_> {
    /// 用途に対応する設定を当てて、候補判定をプラグインごとに組み立てる。
    pub(crate) fn role_filters(&self, role: PatchRole) -> GridRoleFilters<'_> {
        let per_plugin = self
            .patch_plugins
            .plugins()
            .iter()
            .map(|plugin| {
                let roles = &plugin.patch_roles;
                let categories: &[String] = match role {
                    PatchRole::Bass => &roles.bass_patch_categories,
                    PatchRole::Arpeggio => &roles.arpeggio_patch_categories,
                    PatchRole::Kick
                    | PatchRole::Snare
                    | PatchRole::HiHat
                    | PatchRole::Percussion => &roles.drum_patch_categories,
                    // Free は「chord 候補を避ける」判定なので、見るのは chord のカテゴリ。
                    PatchRole::Chord | PatchRole::Free => &roles.chord_patch_categories,
                };
                (categories, role_keywords(roles, role))
            })
            .collect();
        GridRoleFilters {
            role,
            plugins: self.patch_plugins,
            per_plugin,
        }
    }

    pub(crate) fn poly_lookup(&self) -> PolyLookup<'_> {
        PolyLookup(self.voicing)
    }

    /// 用途に合う patch の表示名一覧。PATCH 欄の wheel が引く袋の中身そのもの。
    ///
    /// wheel も一覧フィルタもこれを通るので、「設定を変えたらどの行の候補が
    /// 何件になるか」を画面を起動せずに確かめられる（`cmrt patch-roles`）。
    pub fn role_candidates(&self, role: PatchRole) -> Vec<&str> {
        candidates_for_role(
            self.patches(),
            &self.role_filters(role),
            &self.poly_lookup(),
        )
    }
}

/// 用途に対応する patch 名キーワード。打楽器以外は空。
///
/// percussion は「他の3役に取られなかった残り全部」なので、渡すのは自分のぶんでは
/// なく他3役ぶんの合併になる。
///
/// 照合相手が小文字化した patch パスなので、ここで小文字へ揃える
/// （config.toml に大文字で書かれていても当たるように）。
fn role_keywords(roles: &PatchRoles, role: PatchRole) -> Vec<String> {
    let sources: &[&[String]] = match role {
        PatchRole::Kick => &[roles.kick_patch_keywords.as_slice()],
        PatchRole::Snare => &[roles.snare_patch_keywords.as_slice()],
        PatchRole::HiHat => &[roles.hihat_patch_keywords.as_slice()],
        PatchRole::Percussion => &[
            roles.kick_patch_keywords.as_slice(),
            roles.snare_patch_keywords.as_slice(),
            roles.hihat_patch_keywords.as_slice(),
        ],
        PatchRole::Chord | PatchRole::Bass | PatchRole::Arpeggio | PatchRole::Free => &[],
    };
    sources
        .iter()
        .flat_map(|keywords| keywords.iter())
        .map(|keyword| keyword.to_lowercase())
        .collect()
}
