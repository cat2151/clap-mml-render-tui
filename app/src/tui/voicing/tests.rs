use super::*;
use crate::voicing_sources::VoicingSourceRefresh;

/// カタログは開発機のインストール状況で変わるので、テストは `Config` を通さず
/// カタログを手で並べる。
fn state(plugins: &[CatalogPlugin]) -> VoicingState {
    VoicingState::new(
        VoicingCache::default(),
        VoicingLayers::default(),
        VoicingSourceRefresh::disabled(),
        VoicingPolicies {
            plugins: PatchPlugins::from_catalog(plugins.to_vec()),
        },
    )
}

fn catalog_plugin(plugin_id: &str, plugin_path: &str) -> CatalogPlugin {
    CatalogPlugin {
        name: String::new(),
        plugin_path: plugin_path.to_string(),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        patch_roles: cmrt_runtime::PatchRoles::default(),
    }
}

fn surge_plugin() -> CatalogPlugin {
    catalog_plugin(
        cmrt_runtime::SURGE_XT_PLUGIN_ID,
        cmrt_runtime::default_plugin_path(),
    )
}

fn dexed_plugin() -> CatalogPlugin {
    catalog_plugin(
        cmrt_runtime::DEXED_PLUGIN_ID,
        cmrt_runtime::default_dexed_plugin_path(),
    )
}

fn floe_plugin() -> CatalogPlugin {
    catalog_plugin(
        cmrt_runtime::FLOE_PLUGIN_ID,
        cmrt_runtime::default_floe_plugin_path(),
    )
}

/// `.vvp` を実際に置いた音色置き場ごと用意する。Vaporizer2 の判定は
/// **音色ファイルの中身**を読むので、カタログを並べるだけでは何も決まらない。
struct Vaporizer2Presets {
    root: std::path::PathBuf,
}

impl Vaporizer2Presets {
    fn new(label: &str) -> Self {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("cmrt_voicing_vvp_{label}_{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn write(&self, name: &str, poly_mode: &str) {
        let xml = format!(
            "<VASTvaporizer2 PatchVersion=\"VASTVaporizerParamsV2.20000\" PatchName=\"{name}\"              PatchCategory=\"PD\">
<PARAM id=\"m_uPolyMode\" text=\"{poly_mode}\"/>
             </VASTvaporizer2>
"
        );
        std::fs::write(self.root.join(name), xml).unwrap();
    }

    fn plugin(&self) -> CatalogPlugin {
        CatalogPlugin {
            base: Some(self.root.to_string_lossy().into_owned()),
            dirs: vec![self.root.to_string_lossy().into_owned()],
            ..catalog_plugin(
                cmrt_runtime::VAPORIZER2_PLUGIN_ID,
                r"C:\Program Files\Common Files\CLAP\VASTvaporizer2.clap",
            )
        }
    }
}

impl Drop for Vaporizer2Presets {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn surge_leaves_unknown_patches_undecided() {
    assert_eq!(state(&[surge_plugin()]).resolve("Keys/Unknown.fxp"), None);
}

#[test]
fn plugins_without_patch_level_data_are_all_poly() {
    let state = state(&[dexed_plugin()]);
    assert_eq!(
        state.resolve("SynprezFM/SynprezFM_01.syx/00 Say Again."),
        Some(PatchVoicing::Poly)
    );
    // 名前を知らない patch でも同じ。判定していないのではなく、poly と決めている。
    assert_eq!(state.resolve(""), Some(PatchVoicing::Poly));
}

/// カタログにプラグインが 1 つだけなら、判定方針は全 patch で同じ。
/// `.syx` 形式の patch 文字列を渡しても既定プラグインの方針へ落ちる。
#[test]
fn a_single_plugin_catalog_uses_one_policy_for_every_patch() {
    let surge = state(&[surge_plugin()]);

    assert_eq!(surge.resolve("Keys/Unknown.fxp"), None);
    assert_eq!(surge.resolve("Dexed_01.syx/00 Bell"), None);
}

/// 混在カタログでは方針が patch ごとに変わる。`.fxp` は Surge の層から引き（未判定なら
/// `None`）、cartridge は poly と決める。
#[test]
fn a_mixed_catalog_switches_policy_per_patch() {
    let mixed = state(&[surge_plugin(), dexed_plugin()]);

    assert_eq!(mixed.resolve("Keys/Unknown.fxp"), None);
    assert_eq!(
        mixed.resolve("Dexed_01.syx/00 Bell"),
        Some(PatchVoicing::Poly)
    );
    // 音色を無指定にした行が鳴るのは既定プラグイン（先頭）。
    assert_eq!(mixed.resolve(""), None);
}

/// Vaporizer2 は poly へ倒さない。**Mono と書いてある音色は Mono** と決める。
/// ここを `AssumePoly` にすると、出荷プリセット 460 件のうち 144 件ある Mono が
/// 和音行の候補へ出て、鳴らすと最後の 1 音しか出ない。
#[test]
fn a_vvp_patch_reads_its_poly_mode_from_the_file() {
    let presets = Vaporizer2Presets::new("policy");
    presets.write("PD Wide.vvp", "Poly16");
    presets.write("LD Screamer.vvp", "Mono");
    let state = state(&[presets.plugin()]);

    assert_eq!(state.resolve("PD Wide.vvp"), Some(PatchVoicing::Poly));
    assert_eq!(state.resolve("LD Screamer.vvp"), Some(PatchVoicing::Mono));
}

/// 3 プラグイン混在。`.fxp` は共有 JSON の層（未判定なら `None`）、cartridge は poly、
/// `.vvp` はファイルの中身。**方針が 1 つでも取り違うと和音行の候補が壊れる。**
#[test]
fn a_three_plugin_catalog_reads_each_patch_form_its_own_way() {
    let presets = Vaporizer2Presets::new("mixed");
    presets.write("LD Screamer.vvp", "Mono");
    let state = state(&[surge_plugin(), dexed_plugin(), presets.plugin()]);

    assert_eq!(state.resolve("Keys/Unknown.fxp"), None);
    assert_eq!(
        state.resolve("Dexed_01.syx/00 Bell"),
        Some(PatchVoicing::Poly)
    );
    assert_eq!(state.resolve("LD Screamer.vvp"), Some(PatchVoicing::Mono));
}

#[test]
fn floe_presets_use_assume_poly_in_a_mixed_catalog() {
    let mixed = state(&[surge_plugin(), dexed_plugin(), floe_plugin()]);

    assert_eq!(
        mixed.resolve("Celtic Harp/Realistic.floe-preset"),
        Some(PatchVoicing::Poly)
    );
    assert_eq!(
        VoicingPolicy::for_plugin(&floe_plugin()),
        VoicingPolicy::AssumePoly
    );
}

/// カタログに Vaporizer2 が居なければ `.vvp` は既定プラグインの方針へ落ちる
/// （既存の倒れ方を変えない）。ファイルを開きに行かないので置き場も要らない。
#[test]
fn a_vvp_patch_without_vaporizer2_in_the_catalog_falls_back_to_the_default_policy() {
    assert_eq!(
        state(&[dexed_plugin()]).resolve("PD Wide.vvp"),
        Some(PatchVoicing::Poly)
    );
    assert_eq!(state(&[surge_plugin()]).resolve("PD Wide.vvp"), None);
}

/// 先読みは `.vvp` だけを数える。数えた件数がそのまま「読んだファイル数」。
#[test]
fn prefetching_reports_how_many_vvp_files_it_read() {
    let presets = Vaporizer2Presets::new("prefetch");
    presets.write("PD Wide.vvp", "Poly16");
    let state = state(&[surge_plugin(), presets.plugin()]);

    let pairs = [
        ("PD Wide.vvp".to_string(), "pd wide.vvp".to_string()),
        ("Keys/Bright.fxp".to_string(), "keys/bright.fxp".to_string()),
    ];
    assert_eq!(state.prefetch_vvp_voicings(&pairs), 1);

    // 先読み済みなら、音色ファイルを消しても答えは変わらない。
    std::fs::remove_file(presets.root.join("PD Wide.vvp")).unwrap();
    assert_eq!(state.resolve("PD Wide.vvp"), Some(PatchVoicing::Poly));
}

/// 診断表示（`cmrt patch-roles`）はプラグインごとに方針名を出す。
#[test]
fn every_policy_has_its_own_label() {
    let presets = Vaporizer2Presets::new("label");
    let labels = [surge_plugin(), dexed_plugin(), presets.plugin()]
        .iter()
        .map(|plugin| VoicingPolicy::for_plugin(plugin).label())
        .collect::<Vec<_>>();

    assert!(labels[0].starts_with("Sources"));
    assert!(labels[1].starts_with("AssumePoly"));
    assert!(labels[2].starts_with("VvpHeader"));
}
