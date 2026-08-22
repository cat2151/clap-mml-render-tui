//! `cmrt patch-roles`: grid sequencer の各行に patch の候補が出るかを、画面を起動せずに調べる。
//!
//! PATCH 欄の wheel が引く候補は「config の用途別カテゴリ」「patch 一覧」
//! 「patch ごとの mono/poly 判定」の3つで決まる。プラグインを切り替えたときに
//! どれか1つでも噛み合わないと wheel が無反応になるが、それを目視で確かめるには
//! 画面を開いて全行の wheel を回すしかなかった。ここはその確認を機械化する。
//!
//! 候補の判定は [`cmrt_grid_sequencer::GridSequencerContext::role_candidates`]
//! ＝ wheel 本体と同じ述語を通す。voicing の解決も TUI と同じ
//! [`super::voicing::VoicingState`] を通すので、画面と食い違わない。

use anyhow::Result;

use super::grid_sequencer::{
    row_patch_role, DrumRole, GridPatchLoad, GridSequencerContext, PatchRole, ARPEGGIO_ROW,
    BASS_ROW, CHORD_ROW, FIRST_DRUM_ROW, FULL_DRUM_TRACK_COUNT,
};
use super::voicing::{VoicingPolicies, VoicingPolicy, VoicingState};
use crate::config::Config;
use cmrt_tui_core::patch_plugins::PatchPlugins;

/// 候補が0件の行が1つでもあれば `Err`。スクリプトから終了コードで判定できるようにするため。
pub fn run_patch_role_report(cfg: &Config) -> Result<()> {
    let pairs = crate::patches::collect_patch_pairs(cfg)?;
    let patch_plugins = PatchPlugins::from_config(cfg);
    // TUI 起動時と同じ経路で voicing 判定を組む（Surge なら共有 JSON の取得を待つ）。
    let source_refresh = crate::voicing_sources::VoicingSourceRefresh::spawn(cfg);
    let layers = source_refresh.load_for_keyboard();
    let voicing = VoicingState::new(
        crate::history::load_voicing_cache(),
        layers,
        source_refresh,
        VoicingPolicies::from_config(cfg),
    );
    // `.vvp` の mono/poly は音色ファイルの先頭に書いてある。ここで先に読んでおくと、
    // 下の候補数え上げが 1 音色 1 回の読みで済む（画面側は起動時に同じことをする）。
    voicing.prefetch_vvp_voicings(&pairs);
    let chord_catalog = cmrt_chord::ChordProgressionCatalog::default();
    let ctx = GridSequencerContext {
        patch_dirs_configured: crate::patches::has_configured_patch_dirs(cfg),
        patch_load: GridPatchLoad::Ready(&pairs),
        chord_catalog: &chord_catalog,
        voicing: &voicing,
        patch_plugins: &patch_plugins,
        chord_source_updated: false,
    };

    print_plugin_section(cfg, &patch_plugins);
    print_patch_section(cfg, pairs.len());
    print_filter_section(&patch_plugins);
    print_role_section(&ctx, &patch_plugins);
    let empty_rows = print_row_section(&ctx);

    println!();
    if pairs.is_empty() {
        anyhow::bail!(
            "patch が1件も読み込めていません。config.toml の patches_dirs（プロファイルを使う場合は [plugins.*] の patches_dirs）を確認してください。"
        );
    }
    if !empty_rows.is_empty() {
        anyhow::bail!(
            "候補が0件の行があります: {}。この行では PATCH 欄の wheel が無反応になります。",
            empty_rows.join(" / ")
        );
    }
    println!("判定: すべての行に候補があります。PATCH 欄の wheel はどの行でも回ります。");
    Ok(())
}

fn print_plugin_section(cfg: &Config, patch_plugins: &PatchPlugins) {
    println!("[プラグイン]");
    println!(
        "  active_plugin : {}",
        optional(cfg.active_plugin.as_deref())
    );
    println!("  plugin_path   : {}", optional_str(&cfg.plugin_path));
    println!("  plugin_id     : {}", optional(cfg.plugin_id.as_deref()));
    println!();
    println!("[カタログに音色を載せるプラグイン（先頭が既定）]");
    for plugin in patch_plugins.plugins() {
        println!("  {}", plugin.name);
        println!("    plugin_path : {}", optional_str(&plugin.plugin_path));
        println!("    Surge XT か : {}", yes_no(plugin.is_surge_xt()));
        println!(
            "    voicing 判定: {}",
            VoicingPolicy::for_plugin(plugin).label()
        );
    }
}

fn print_patch_section(cfg: &Config, count: usize) {
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
    println!("  読み込み件数  : {count}");
}

/// 用途別の絞り込みはプラグインごとに違う（Surge のカテゴリを cartridge へ当てると
/// 候補が全滅する）。プロファイル適用後の解決結果をプラグインごとに出す。
fn print_filter_section(patch_plugins: &PatchPlugins) {
    for plugin in patch_plugins.plugins() {
        let roles = &plugin.patch_roles;
        println!();
        println!(
            "[用途別カテゴリ / キーワード（{} / [plugins.*] プロファイル適用後）]",
            plugin.name
        );
        for (label, values) in [
            ("chord_patch_categories   ", &roles.chord_patch_categories),
            ("bass_patch_categories    ", &roles.bass_patch_categories),
            (
                "arpeggio_patch_categories",
                &roles.arpeggio_patch_categories,
            ),
            ("drum_patch_categories    ", &roles.drum_patch_categories),
            ("kick_patch_keywords      ", &roles.kick_patch_keywords),
            ("snare_patch_keywords     ", &roles.snare_patch_keywords),
            ("hihat_patch_keywords     ", &roles.hihat_patch_keywords),
        ] {
            println!("  {label} : {}", list_label(values));
        }
    }
}

/// 全 8 役割の候補数。行の割り当てが変わっても、行が取りうる用途はこの 8 つで尽きる。
///
/// **カタログが混在なら、プラグインごとの内訳も出す。** 合計だけでは
/// 「Vaporizer2 の音色がこの役へ 1 件も出ていない」が見えない（他プラグインの数千件に
/// 埋もれる）。プラグインを足したときに効いたのかどうかは、この内訳が動くかで決まる。
fn print_role_section(ctx: &GridSequencerContext<'_>, patch_plugins: &PatchPlugins) {
    println!();
    println!("[用途ごとの候補数]");
    for role in ALL_ROLES {
        let candidates = ctx.role_candidates(role);
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

/// 候補をプラグインごとに数えて `名前 件数` の 1 行にする。
///
/// 引き分けは patch 一覧のフィルタ・PATCH 欄の wheel と同じ
/// [`PatchPlugins::index_for_patch`] を通す。別の判定を書くと、内訳は正しく見えるのに
/// 画面では違うプラグインへ送られている、という食い違いが起きる。
fn per_plugin_counts(candidates: &[&str], patch_plugins: &PatchPlugins) -> String {
    let mut counts = vec![0usize; patch_plugins.plugins().len()];
    for display in candidates {
        counts[patch_plugins.index_for_patch(display)] += 1;
    }
    patch_plugins
        .plugins()
        .iter()
        .zip(counts)
        .map(|(plugin, count)| format!("{} {count}", plugin.name))
        .collect::<Vec<_>>()
        .join(" / ")
}

/// 行ごとの用途と候補数。候補が0件だった行のラベルを返す。
///
/// drum 行の割り当ては track 数だけで決まる（`state::drum` の `drum_role_for`）。
/// ここは 4 role を過不足なく持つ [`FULL_DRUM_TRACK_COUNT`] の構成で表を出す。
/// track 4（drum 行が1つだけ）のときは役割が抽選になるが、抽選先は 4 role の
/// どれかなので、上の「用途ごとの候補数」で全て押さえてある。
fn print_row_section(ctx: &GridSequencerContext<'_>) -> Vec<String> {
    let mut empty = Vec::new();
    for chord_on in [true, false] {
        println!();
        println!(
            "[行ごとの用途（chord mode {} / track {FULL_DRUM_TRACK_COUNT}）]",
            if chord_on { "ON" } else { "OFF" }
        );
        for row in 0..FULL_DRUM_TRACK_COUNT {
            let drum = row
                .checked_sub(FIRST_DRUM_ROW)
                .and_then(|index| DrumRole::ALL.get(index))
                .copied();
            let role = row_patch_role(row, chord_on, drum);
            let count = ctx.role_candidates(role).len();
            println!(
                "  行{row} {:<10} {:<11} {count:>6} 件",
                row_label(row, chord_on, drum),
                role_label(role)
            );
            if count == 0 {
                empty.push(format!(
                    "chord mode {} の行{row}（{}）",
                    if chord_on { "ON" } else { "OFF" },
                    role_label(role)
                ));
            }
        }
    }
    empty
}

const ALL_ROLES: [PatchRole; 8] = [
    PatchRole::Chord,
    PatchRole::Bass,
    PatchRole::Arpeggio,
    PatchRole::Free,
    PatchRole::Kick,
    PatchRole::Snare,
    PatchRole::HiHat,
    PatchRole::Percussion,
];

fn role_label(role: PatchRole) -> &'static str {
    match role {
        PatchRole::Chord => "Chord",
        PatchRole::Bass => "Bass",
        PatchRole::Arpeggio => "Arpeggio",
        PatchRole::Free => "Free",
        PatchRole::Kick => "Kick",
        PatchRole::Snare => "Snare",
        PatchRole::HiHat => "HiHat",
        PatchRole::Percussion => "Percussion",
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

fn yes_no(value: bool) -> &'static str {
    if value {
        "はい"
    } else {
        "いいえ"
    }
}

fn list_label(values: &[String]) -> String {
    if values.is_empty() {
        "(空 = 絞らない)".to_string()
    } else {
        values.join(", ")
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
