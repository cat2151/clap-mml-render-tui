//! Dexed の cartridge program のうち、同じ音色が何件重複しているかを数える診断（`cmrt dexed-duplicates`）。
//!
//! 「同じ音色」は、play server が Dexed へ送る single voice SysEx（163 bytes）のバイト一致で判定する。
//! この SysEx は program 名 10 文字も含むので、パラメータが同じで名前だけ違う program は
//! 別の音色として数えられる。それも見えるよう、名前を除いたパラメータ部の一致でも数える。
//!
//! 絞り込み条件は patch 選択画面と同じ規則（[`text_filter`]）で、Dexed の display 文字列に当てる。
//! 音色一覧は同じ音をまとめて返すので、ここはまとめる前の全 program（[`cmrt_core::collect_patches`]）を見る。

use anyhow::{Context, Result};
use cmrt_core::dx7::{
    parse_cartridge_patch_path, parse_dx7_cartridge, single_voice_sysex, voice_params_without_name,
    Dx7Cartridge, DEXED_PLUGIN_ID,
};
use cmrt_runtime::{catalog_plugins, CatalogPlugin};
use cmrt_tui_core::text_filter;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::PathBuf;

use crate::config::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DexedDuplicatesRequest {
    pub config: Option<PathBuf>,
    /// patch 選択画面と同じ絞り込み条件。空なら Dexed の全 program。
    pub condition: String,
}

/// 調べた program 1 件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DexedProgram {
    pub display: String,
    pub sysex: Vec<u8>,
    /// 名前を除いた voice パラメータ。音色一覧が同じ音をまとめる鍵と同じもの。
    pub params: Vec<u8>,
}

pub fn run(cfg: &Config, request: &DexedDuplicatesRequest) -> Result<()> {
    let condition = text_filter::compile_condition(&request.condition)
        .map_err(|error| anyhow::anyhow!("絞り込み条件を正規表現として読めない: {error}"))?;
    let plugins: Vec<CatalogPlugin> = catalog_plugins(cfg)
        .into_iter()
        .filter(|plugin| plugin.plugin_id.as_deref() == Some(DEXED_PLUGIN_ID))
        .collect();
    anyhow::ensure!(
        !plugins.is_empty(),
        "Dexed が音色 catalog に載っていない（plugin_id = '{DEXED_PLUGIN_ID}'）"
    );

    let mut cartridges = CartridgeCache::default();
    let mut programs = Vec::new();
    for plugin in &plugins {
        for dir in &plugin.dirs {
            let base = plugin.base.as_deref().unwrap_or(dir);
            for path in cmrt_core::collect_patches(dir)? {
                let display = cmrt_core::to_relative(base, &path);
                if !text_filter::matches_any_field(&condition, &[display.to_lowercase().as_str()]) {
                    continue;
                }
                if let Some((sysex, params)) = cartridges.voice(&path.to_string_lossy()) {
                    programs.push(DexedProgram {
                        display,
                        sysex,
                        params,
                    });
                }
            }
        }
    }

    print!(
        "{}",
        report(&request.condition, &programs, &cartridges.errors)
    );
    Ok(())
}

/// 同じ cartridge を 32 回読まないための cache。読めなかった cartridge は理由を残す。
#[derive(Default)]
struct CartridgeCache {
    loaded: HashMap<String, Option<Dx7Cartridge>>,
    errors: Vec<String>,
}

impl CartridgeCache {
    /// 送る SysEx と、名前を除いた voice パラメータ。
    fn voice(&mut self, patch_path: &str) -> Option<(Vec<u8>, Vec<u8>)> {
        let patch = match parse_cartridge_patch_path(patch_path) {
            Ok(patch) => patch,
            Err(error) => {
                self.errors.push(format!("{error:#}"));
                return None;
            }
        };
        if !self.loaded.contains_key(&patch.cartridge_path) {
            let loaded = std::fs::read(&patch.cartridge_path)
                .with_context(|| format!("cartridge を読めない '{}'", patch.cartridge_path))
                .and_then(parse_dx7_cartridge);
            let cartridge = match loaded {
                Ok(cartridge) => Some(cartridge),
                Err(error) => {
                    self.errors.push(format!("{error:#}"));
                    None
                }
            };
            self.loaded.insert(patch.cartridge_path.clone(), cartridge);
        }
        let cartridge = self.loaded[&patch.cartridge_path].as_ref()?;
        Some((
            single_voice_sysex(cartridge, patch.program_index),
            voice_params_without_name(cartridge, patch.program_index),
        ))
    }
}

/// 2 件以上ある group だけを、大きい順（同数なら先頭 display 順）に返す。
fn duplicate_groups<'a, K: Hash + Eq>(
    programs: &'a [DexedProgram],
    key: impl Fn(&'a DexedProgram) -> K,
) -> Vec<Vec<usize>> {
    let mut by_key: HashMap<K, Vec<usize>> = HashMap::new();
    for (index, program) in programs.iter().enumerate() {
        by_key.entry(key(program)).or_default().push(index);
    }
    let mut groups: Vec<Vec<usize>> = by_key
        .into_values()
        .filter(|group| group.len() >= 2)
        .collect();
    groups.sort_by(|a, b| {
        b.len()
            .cmp(&a.len())
            .then_with(|| programs[a[0]].display.cmp(&programs[b[0]].display))
    });
    groups
}

fn exact_key(program: &DexedProgram) -> &[u8] {
    &program.sysex
}

fn params_key(program: &DexedProgram) -> &[u8] {
    &program.params
}

pub(crate) fn report(condition: &str, programs: &[DexedProgram], errors: &[String]) -> String {
    let exact = duplicate_groups(programs, exact_key);
    let params = duplicate_groups(programs, params_key);
    // パラメータは同じなのに SysEx が割れている group ＝ 名前だけ違う program を含む。
    let name_only: Vec<&Vec<usize>> = params
        .iter()
        .filter(|group| {
            let first = exact_key(&programs[group[0]]);
            group
                .iter()
                .any(|&index| exact_key(&programs[index]) != first)
        })
        .collect();

    let mut out = format!(
        "dexed-duplicates condition=\"{condition}\" programs={} cartridge_errors={}\n",
        programs.len(),
        errors.len()
    );
    out += &summary_line("SysEx 完全一致", programs.len(), &exact);
    out += &summary_line("名前を除くパラメータ一致", programs.len(), &params);
    out += &format!("名前だけ違う program を含む group={}\n", name_only.len());

    for (number, group) in exact.iter().enumerate() {
        out += &format!(
            "\n[SysEx 完全一致] group {}: {} programs\n",
            number + 1,
            group.len()
        );
        out += &group_lines(programs, group);
    }
    for (number, group) in name_only.iter().enumerate() {
        out += &format!(
            "\n[名前だけ違う] group {}: {} programs\n",
            number + 1,
            group.len()
        );
        out += &group_lines(programs, group);
    }
    for error in errors {
        out += &format!("\n[cartridge error] {error}");
    }
    if !errors.is_empty() {
        out += "\n";
    }
    out
}

/// `unique` は重複を 1 件に数えた種類数、`redundant` は重複で余分な件数。
fn summary_line(label: &str, total: usize, groups: &[Vec<usize>]) -> String {
    let redundant: usize = groups.iter().map(|group| group.len() - 1).sum();
    let percent = if total == 0 {
        0.0
    } else {
        redundant as f64 * 100.0 / total as f64
    };
    format!(
        "{label}: unique={} duplicate_groups={} redundant={redundant} ({percent:.1}%)\n",
        total - redundant,
        groups.len()
    )
}

fn group_lines(programs: &[DexedProgram], group: &[usize]) -> String {
    group
        .iter()
        .map(|&index| format!("  {}\n", programs[index].display))
        .collect()
}

#[cfg(test)]
mod tests;
