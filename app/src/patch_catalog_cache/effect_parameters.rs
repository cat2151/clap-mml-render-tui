//! Surge XT Effects の公式sourceからparameter cacheを生成する。

use anyhow::{Context, Result};
use cmrt_core::surge_fx_preset::{
    decode_param_layouts, PARAM_LAYOUT_CACHE_RELATIVE_PATH, SURGE_FX_PLUGIN_ID,
};
use std::{fs, path::Path};

pub(super) fn build_and_save(app_dir: &Path) -> Result<()> {
    let plugin = cmrt_core::builtin_effect_plugins()
        .into_iter()
        .find(|p| p.plugin_id == SURGE_FX_PLUGIN_ID && Path::new(&p.plugin_path).is_file())
        .context("Surge XT Effects が見つかりません")?;
    let path = app_dir.join(PARAM_LAYOUT_CACHE_RELATIVE_PATH);
    let bytes = cmrt_core::surge_fx_source::prepare_layout_cache(
        &plugin.plugin_path,
        &path,
        &app_dir.join("surge-fx-sources"),
    )
    .context("Surge XT Effects の公式sourceからparameter表を生成できません")?;
    if fs::read(&path).is_ok_and(|old| old == bytes) {
        return Ok(());
    }
    save(&path, &bytes)
}

fn save(path: &Path, bytes: &[u8]) -> Result<()> {
    // 不正な生成物で利用中の cache を置き換えない。
    decode_param_layouts(bytes)?;
    fs::create_dir_all(path.parent().context("parameter cache の親がありません")?)?;
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let result = (|| {
        fs::write(&temporary, bytes)?;
        super::replace_file(&temporary, path)
    })();
    let _ = fs::remove_file(&temporary);
    result.with_context(|| format!("parameter cache を保存できません: {}", path.display()))
}

#[cfg(test)]
mod tests;
