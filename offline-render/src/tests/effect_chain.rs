//! DAW のセル render に effect chain が効くこと、効かない経路では黙って dry にならないこと。
//!
//! 実 plugin が要るものは `#[ignore]` で、instrument の CLAP を環境変数で受ける。
//! effect は組み込み既定パスの catalog（`AudioEffectCatalog::discover`）を使う:
//!
//! ```text
//! CMRT_TEST_SURGE_CLAP=C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap
//! cargo test -p cmrt-offline-render --release effect_chain -- --ignored --nocapture
//! ```

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use cmrt_core::{AudioEffectCatalog, EFFECT_CHAIN_JSON_KEY};
use cmrt_runtime::{CatalogPlugin, Config, SURGE_XT_PLUGIN_ID};

use crate::{EffectPlugins, OfflineRenderer, PluginEntries};

const SURGE_CLAP_ENV: &str = "CMRT_TEST_SURGE_CLAP";
const CACHE_PLAYER_PLUGIN_ID: &str = "org.cat2151.cmrt.cache-player";
const CACHE_PLAYER_PLUGIN_PATH: &str = "builtin:org.cat2151.cmrt.cache-player";

/// `build_cell_mml_from_data` が返す形（先頭 JSON の直後に本文）。init セルの JSON だけを
/// 持ち、音色は無指定（既定プラグインの Init Saw）。
const DRY_CELL_MML: &str = "t120o4l4crrr";
const REVERB_CELL_MML: &str = r#"{"effects after instrument":[{"Surge XT Effects preset":"Reverb 1/Cathedral 2.srgfx"}]}t120o4l4crrr"#;

fn config_for_test(plugin_path: &str) -> Arc<Config> {
    let mut cfg: Config = toml::from_str(
        r#"
plugin_path = "(replaced below)"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
offline_render_backend = "in_process"
"#,
    )
    .unwrap();
    cfg.plugin_path = plugin_path.to_string();
    Arc::new(cfg)
}

fn catalog_plugin(name: &str, plugin_path: &str, plugin_id: &str) -> CatalogPlugin {
    CatalogPlugin {
        name: name.to_string(),
        plugin_path: plugin_path.to_string(),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        resolved_patches: None,
        source_notices: Vec::new(),
    }
}

/// render の中間ファイルを実ユーザーの置き場へ書かない。
fn redirect_render_dirs() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "cmrt_offline_render_effect_chain_{}_{suffix}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("CMRT_BASE_DIR", dir);
}

fn renderer_with(plugin: CatalogPlugin, effects: EffectPlugins) -> OfflineRenderer {
    redirect_render_dirs();
    let cfg = config_for_test(&plugin.plugin_path);
    let entry = cmrt_core::load_entry(&plugin.plugin_path).unwrap();
    let entries = PluginEntries::from_loaded(vec![plugin], &[entry]).with_effects(effects);
    OfflineRenderer::new(cfg, entries)
}

/// 組み込みの cache-player を instrument に立てた renderer。DLL 無しで render 経路を通せる。
fn cache_player_renderer(effects: EffectPlugins) -> OfflineRenderer {
    renderer_with(
        catalog_plugin(
            "cache-player",
            CACHE_PLAYER_PLUGIN_PATH,
            CACHE_PLAYER_PLUGIN_ID,
        ),
        effects,
    )
}

fn render_cell(renderer: &OfflineRenderer, mml: &str) -> anyhow::Result<Vec<f32>> {
    let prepared = renderer.prepare_cache_render(mml)?;
    renderer.render_prepared_cache(prepared, None)
}

fn rms_dbfs(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return -f32::INFINITY;
    }
    let sum: f64 = samples.iter().map(|s| f64::from(*s).powi(2)).sum();
    20.0 * ((sum / samples.len() as f64).sqrt() as f32).log10()
}

/// 後半 1 秒（音符が鳴り終わった後）の RMS。リバーブの尻尾はここに出る。
fn tail_dbfs(samples: &[f32], sample_rate: usize) -> f32 {
    let frames = samples.len() / 2;
    rms_dbfs(&samples[(frames - sample_rate) * 2..])
}

#[test]
fn a_cell_without_a_chain_renders_on_a_route_without_effects() {
    let renderer = cache_player_renderer(EffectPlugins::none());

    let samples = render_cell(&renderer, DRY_CELL_MML).unwrap();

    assert!(!samples.is_empty());
}

/// chain 付きの MML が effect を持たない経路に来たら、dry で鳴らさずエラーにする。
#[test]
fn a_chain_is_rejected_on_a_route_without_effects() {
    let renderer = cache_player_renderer(EffectPlugins::none());

    let error = render_cell(&renderer, REVERB_CELL_MML).unwrap_err();

    assert!(
        format!("{error:#}").contains(EFFECT_CHAIN_JSON_KEY),
        "{error:#}"
    );
}

/// catalog に無い effect は render 前にエラーになる。
#[test]
fn a_chain_with_an_unlisted_effect_is_rejected_before_rendering() {
    let renderer =
        cache_player_renderer(EffectPlugins::with_catalog(AudioEffectCatalog::default()));

    let error = render_cell(&renderer, REVERB_CELL_MML).unwrap_err();

    assert!(
        format!("{error:#}").contains("Surge XT Effects preset"),
        "{error:#}"
    );
}

#[test]
#[ignore = "実プラグインが要る（CMRT_TEST_SURGE_CLAP と組み込み既定パスの effect）"]
fn a_daw_cell_with_a_reverb_chain_has_a_tail_and_the_dry_cell_does_not() {
    let surge = std::env::var(SURGE_CLAP_ENV).unwrap_or_else(|_| {
        panic!("{SURGE_CLAP_ENV} にパスを設定してからこのテストを実行すること")
    });
    let effects = EffectPlugins::discover();
    let catalog = effects.catalog().unwrap();
    assert!(
        catalog
            .plugins()
            .iter()
            .any(|plugin| plugin.json_key == "Surge XT Effects preset"),
        "Surge XT Effects が catalog に無い: {:?}",
        catalog.skipped()
    );
    let renderer = renderer_with(
        catalog_plugin("Surge XT", &surge, SURGE_XT_PLUGIN_ID),
        effects,
    );

    let dry = render_cell(&renderer, DRY_CELL_MML).unwrap();
    let started = std::time::Instant::now();
    let wet = render_cell(&renderer, REVERB_CELL_MML).unwrap();
    let wet_elapsed = started.elapsed();

    let dry_tail = tail_dbfs(&dry, 48_000);
    let wet_tail = tail_dbfs(&wet, 48_000);
    println!(
        "dry: rms {:.1} dBFS / tail {dry_tail:.1} dBFS, wet: rms {:.1} dBFS / tail {wet_tail:.1} dBFS ({wet_elapsed:?})",
        rms_dbfs(&dry),
        rms_dbfs(&wet)
    );
    assert_eq!(dry.len(), wet.len(), "chain は長さを変えない");
    assert!(rms_dbfs(&dry) > -60.0, "dry が無音");
    assert!(rms_dbfs(&wet) > -60.0, "wet が無音");
    assert!(
        wet_tail > dry_tail + 6.0,
        "リバーブの尻尾が出ていない: dry {dry_tail} / wet {wet_tail}"
    );
    assert!(wet_tail > -60.0, "尻尾が可聴でない: {wet_tail}");
}
