//! auto reverb のルール（on/off を含む）を history の保存先から読み書きする。

use cmrt_offline_render::EffectPlugins;
use cmrt_patch_select::auto_reverb::AutoReverbRules;

/// 保存済みのルール。保存が無い・読めないときは既定のルール。
pub(in crate::tui) fn load_auto_reverb_rules(effect_plugins: &EffectPlugins) -> AutoReverbRules {
    cmrt_history::load_auto_reverb_settings()
        .map(|settings| {
            AutoReverbRules::from_saved(settings.enabled, &settings.rules, effect_plugins.catalog())
        })
        .unwrap_or_default()
}

/// 保存する。失敗は log に残すだけで、画面の操作は続ける。
pub(in crate::tui) fn save_auto_reverb_rules(rules: &AutoReverbRules) {
    let settings = cmrt_history::AutoReverbSettings {
        enabled: rules.enabled(),
        rules: rules.to_saved(),
    };
    if let Err(error) = cmrt_history::save_auto_reverb_settings(&settings) {
        crate::logging::global_log_sink(&format!("auto reverb の保存に失敗: {error:#}"));
    }
}
