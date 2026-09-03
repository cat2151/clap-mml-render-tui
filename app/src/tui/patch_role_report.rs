//! `cmrt patch-roles`: 共有Role分類とGrid各行の候補を画面なしで診断する。

use std::borrow::Cow;

use anyhow::Result;
use cmrt_patches::{PatchRole, PatchRoleIndex, PatchRoleInput};
use cmrt_tui_core::patch_plugins::PatchPlugins;

use super::grid_sequencer::{
    candidates_for_purpose, row_patch_purpose, DrumRole, GridPatchLoad, GridPatchPurpose,
    GridSequencerContext, ARPEGGIO_ROW, BASS_ROW, CHORD_ROW, FIRST_DRUM_ROW, FULL_DRUM_TRACK_COUNT,
};
use super::voicing::{VoicingPolicies, VoicingPolicy, VoicingState};
use crate::config::Config;

/// 候補が0件の行が1つでもあれば`Err`。スクリプトから終了コードで判定できる。
pub fn run_patch_role_report(cfg: &Config) -> Result<()> {
    let pairs = crate::patches::collect_patch_pairs(cfg)?;
    let patch_plugins = PatchPlugins::from_config(cfg);
    let selector_categories = selector_categories(&pairs, &patch_plugins);
    let patch_roles = PatchRoleIndex::build(
        pairs.iter().zip(&selector_categories).map(
            |((display, normalized_display), selector_category)| PatchRoleInput {
                display,
                normalized_display,
                selector_category: selector_category.as_deref(),
            },
        ),
        &crate::history::load_mml_patch_filter_presets(),
    );

    let source_refresh = crate::voicing_sources::VoicingSourceRefresh::spawn(cfg);
    let layers = source_refresh.load_for_keyboard();
    let voicing = VoicingState::new(
        crate::history::load_voicing_cache(),
        layers,
        source_refresh,
        VoicingPolicies::from_config(cfg),
    );
    voicing.prefetch_catalog_voicings(&pairs);
    let chord_catalog = cmrt_chord::ChordProgressionCatalog::default();
    let ctx = GridSequencerContext {
        patch_dirs_configured: crate::patches::has_configured_patch_dirs(cfg),
        patch_load: GridPatchLoad::Ready(&pairs),
        load_measurements: None,
        chord_catalog: &chord_catalog,
        voicing: &voicing,
        patch_roles: Cow::Borrowed(&patch_roles),
        chord_source_updated: false,
        catalog_notes: &[],
    };

    print_plugin_section(cfg, &patch_plugins);
    print_skipped_section(&cmrt_runtime::skipped_catalog_plugins(cfg));
    print_patch_section(cfg, &pairs, &patch_plugins);
    print_role_section(&patch_roles, &patch_plugins);
    let empty_rows = print_row_section(&ctx);

    println!();
    if pairs.is_empty() {
        anyhow::bail!(
            "patch が1件も読み込めていません。config.toml の patches_dirs を確認してください。"
        );
    }
    if !empty_rows.is_empty() {
        anyhow::bail!(
            "候補が0件の行があります: {}。空の用途へALLからfallbackはしません。",
            empty_rows.join(" / ")
        );
    }
    println!("判定: すべての行に用途別候補があります。");
    Ok(())
}

/// 画面と同じ`selector_category`を、pluginのadapterから引き直す。
///
/// **ここを`None`で済ませてはいけない。** 実画面（[`cmrt_tui_core::patch_load::PatchCatalogSnapshot`]）は
/// catalog cacheのcategoryを渡して分類しており、categoryは表示名より先に評価される。
/// この診断だけcategory抜きで分類すると、`Basses/Hate.fxp`が`\bhat`に釣られてDrumへ
/// 落ちるような、**画面では起きない誤分類を報告してしまう**。
///
/// categoryを持たないplugin（sfz・Dexed・Floe）は画面側でも`None`なので、そのまま`None`。
fn selector_categories(
    pairs: &[(String, String)],
    patch_plugins: &PatchPlugins,
) -> Vec<Option<String>> {
    pairs
        .iter()
        .map(|(display, _)| {
            let index = patch_plugins.index_for_patch(display).ok()?;
            patch_plugins.audio_info(index)?.selector_category(display)
        })
        .collect()
}

fn print_plugin_section(cfg: &Config, patch_plugins: &PatchPlugins) {
    println!("[プラグイン]");
    println!("  primary_plugin: Surge XT (固定)");
    println!("  plugin_path   : {}", optional_str(&cfg.plugin_path));
    println!("  plugin_id     : {}", optional(cfg.plugin_id.as_deref()));
    println!();
    println!("[カタログに音色を載せるプラグイン（先頭が既定）]");
    for (index, plugin) in patch_plugins.plugins().iter().enumerate() {
        println!("  {}", plugin.name);
        println!("    plugin_path : {}", optional_str(&plugin.plugin_path));
        let source = patch_plugins
            .audio_info(index)
            .map(|info| format!("{:?}", info.voicing_source()))
            .unwrap_or_else(|| "unknown".to_string());
        println!("    voicing source: {source}");
        println!(
            "    voicing 判定: {}",
            VoicingPolicy::for_plugin(plugin).label()
        );
    }
}

fn print_skipped_section(skipped: &[cmrt_runtime::SkippedCatalogPlugin]) {
    println!();
    println!("[カタログから外したプラグイン]");
    for line in skipped_section_lines(skipped) {
        println!("  {line}");
    }
}

fn skipped_section_lines(skipped: &[cmrt_runtime::SkippedCatalogPlugin]) -> Vec<String> {
    if skipped.is_empty() {
        return vec![
            "なし（インストール済みのプラグインはすべてカタログに載っています）".to_string(),
        ];
    }
    skipped.iter().map(|plugin| plugin.notice_line()).collect()
}

fn print_patch_section(cfg: &Config, pairs: &[(String, String)], patch_plugins: &PatchPlugins) {
    println!();
    println!("[patch 一覧]");
    println!(
        "  patches_dirs  : {}",
        if crate::patches::has_configured_patch_dirs(cfg) {
            "設定あり"
        } else {
            "未設定"
        }
    );
    println!("  読み込み件数  : {}", pairs.len());
    let displays = pairs
        .iter()
        .map(|(display, _)| display.as_str())
        .collect::<Vec<_>>();
    println!(
        "  プラグイン別  : {}",
        per_plugin_counts(&displays, patch_plugins)
    );
}

fn print_role_section(patch_roles: &PatchRoleIndex, patch_plugins: &PatchPlugins) {
    println!();
    println!("[catalog Roleごとの件数]");
    for role in PatchRole::ALL {
        let candidates = patch_roles
            .candidates(role)
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        println!(
            "  {:<11} {:>6} 件  {}",
            role_label(role),
            candidates.len(),
            sample_label(&candidates)
        );
        if patch_plugins.plugins().len() > 1 {
            println!(
                "                       内訳: {}",
                per_plugin_counts(&candidates, patch_plugins)
            );
        }
    }
}

fn per_plugin_counts(candidates: &[&str], patch_plugins: &PatchPlugins) -> String {
    let mut counts = vec![0usize; patch_plugins.plugins().len()];
    for display in candidates {
        if let Ok(index) = patch_plugins.index_for_patch(display) {
            counts[index] += 1;
        }
    }
    patch_plugins
        .plugins()
        .iter()
        .zip(counts)
        .map(|(plugin, count)| format!("{} {count}", plugin.name))
        .collect::<Vec<_>>()
        .join(" / ")
}

fn print_row_section(ctx: &GridSequencerContext<'_>) -> Vec<String> {
    let mut empty = Vec::new();
    for chord_on in [true, false] {
        println!();
        println!(
            "[行ごとの用途（chord mode {} / track {FULL_DRUM_TRACK_COUNT}）]",
            if chord_on { "ON" } else { "OFF" }
        );
        let reserved = if chord_on {
            vec![
                PatchRole::Chord,
                PatchRole::Bass,
                PatchRole::Lead,
                PatchRole::Drum,
            ]
        } else {
            vec![PatchRole::Drum]
        };
        for row in 0..FULL_DRUM_TRACK_COUNT {
            let drum = row
                .checked_sub(FIRST_DRUM_ROW)
                .and_then(|index| DrumRole::ALL.get(index))
                .copied();
            let purpose = row_patch_purpose(row, chord_on, drum);
            let count = candidates_for_purpose(ctx, purpose, &reserved).len();
            println!(
                "  行{row} {:<10} {:<11} {count:>6} 件",
                row_label(row, chord_on, drum),
                purpose_label(purpose)
            );
            if count == 0 {
                empty.push(format!(
                    "chord mode {} の行{row}（{}）",
                    if chord_on { "ON" } else { "OFF" },
                    purpose_label(purpose)
                ));
            }
        }
    }
    empty
}

fn role_label(role: PatchRole) -> &'static str {
    match role {
        PatchRole::Bass => "Bass",
        PatchRole::Chord => "Chord",
        PatchRole::Lead => "Lead",
        PatchRole::Drum => "Drum",
        PatchRole::Triggered => "Triggered",
        PatchRole::Etc => "Etc",
    }
}

fn purpose_label(purpose: GridPatchPurpose) -> &'static str {
    match purpose {
        GridPatchPurpose::Note => "NOTE",
        GridPatchPurpose::Chord => "Chord",
        GridPatchPurpose::Bass => "Bass",
        GridPatchPurpose::Arpeggio => "Arpeggio",
        GridPatchPurpose::Kick => "Kick",
        GridPatchPurpose::Snare => "Snare",
        GridPatchPurpose::HiHat => "HiHat",
        GridPatchPurpose::Percussion => "Percussion",
    }
}

fn row_label(row: usize, chord_on: bool, drum: Option<DrumRole>) -> String {
    if let Some(drum) = drum {
        return drum.label().to_string();
    }
    match row {
        CHORD_ROW if chord_on => "CHORD".to_string(),
        BASS_ROW if chord_on => "BASS".to_string(),
        ARPEGGIO_ROW if chord_on => "ARP".to_string(),
        _ => "-".to_string(),
    }
}

fn sample_label(candidates: &[&str]) -> String {
    if candidates.is_empty() {
        return String::new();
    }
    let head = candidates
        .iter()
        .take(2)
        .copied()
        .collect::<Vec<_>>()
        .join(" | ");
    if candidates.len() > 2 {
        format!("例: {head} ...")
    } else {
        format!("例: {head}")
    }
}

fn optional(value: Option<&str>) -> String {
    value.map_or_else(|| "(未設定)".to_string(), str::to_string)
}

fn optional_str(value: &str) -> &str {
    if value.is_empty() {
        "(未設定)"
    } else {
        value
    }
}

#[cfg(test)]
mod tests;
