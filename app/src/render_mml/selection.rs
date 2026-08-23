//! `render-mml` が鳴らす patch の選択。

use anyhow::Result;
use cmrt_runtime::Config;
use cmrt_tui_core::patch_plugins::PatchPlugins;

use super::RenderMmlRequest;

pub(super) fn requested_patches(
    cfg: &Config,
    catalog: &PatchPlugins,
    request: &RenderMmlRequest,
) -> Result<Vec<Option<String>>> {
    if request.plugin.is_some() && !request.patches.is_empty() {
        anyhow::bail!("--plugin と --patch は併用できません");
    }
    let Some(name) = request.plugin.as_deref() else {
        return Ok(if request.patches.is_empty() {
            vec![None]
        } else {
            request.patches.iter().cloned().map(Some).collect()
        });
    };
    let wanted = normalize_plugin_name(name);
    let index = catalog
        .plugins()
        .iter()
        .position(|plugin| normalize_plugin_name(&plugin.name) == wanted)
        .ok_or_else(|| unknown_plugin_error(name, catalog))?;
    let patches = crate::patches::collect_patch_pairs(cfg)?
        .into_iter()
        .filter_map(|(display, _)| match catalog.index_for_patch(&display) {
            Ok(routed) if routed == index => Some(Ok(Some(display))),
            Ok(_) => None,
            Err(error) => Some(Err(anyhow::Error::new(error))),
        })
        .collect::<Result<Vec<_>>>()?;
    if patches.is_empty() {
        anyhow::bail!(
            "plugin '{}' の patch が 0 件です",
            catalog.plugins()[index].name
        );
    }
    if request.verify {
        verify_routing(catalog, &patches, &wanted)?;
    }
    Ok(patches)
}

fn verify_routing(catalog: &PatchPlugins, patches: &[Option<String>], wanted: &str) -> Result<()> {
    for patch in patches.iter().flatten() {
        let routed = catalog.for_patch(patch).map_err(anyhow::Error::new)?;
        if normalize_plugin_name(&routed.name) != wanted {
            anyhow::bail!(
                "verify 失敗: patch '{patch}' が '{}' へ routing されました",
                routed.name
            );
        }
    }
    Ok(())
}

fn unknown_plugin_error(name: &str, catalog: &PatchPlugins) -> anyhow::Error {
    let available = catalog
        .plugins()
        .iter()
        .map(|plugin| plugin.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    anyhow::anyhow!("plugin '{name}' は共有カタログにありません（利用可能: {available}）")
}

fn normalize_plugin_name(name: &str) -> String {
    name.chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '-' && *ch != '_')
        .flat_map(char::to_lowercase)
        .collect()
}
