use super::*;

#[test]
fn default_config_content_uses_48000_sample_rate() {
    let content = default_config_content();

    assert!(
        content.contains("sample_rate = 48000"),
        "default config の sample_rate は 48000Hz であるべき: {}",
        content
    );
}

#[test]
fn default_config_content_uses_patches_dirs_key() {
    let content = default_config_content();

    assert!(
        content.contains("patches_dirs"),
        "default config は patches_dirs を案内するべき: {}",
        content
    );
}

#[test]
fn default_config_has_no_retired_top_level_plugin_settings() {
    let content = default_config_content();
    assert!(!content.contains("active_plugin"), "{content}");

    let table: toml::Table = toml::from_str(&content).unwrap();
    for key in [
        "plugin_path",
        "plugin_id",
        "patches_dirs",
        "chord_patch_categories",
        "bass_patch_categories",
        "arpeggio_patch_categories",
        "drum_patch_categories",
        "kick_patch_keywords",
        "snare_patch_keywords",
        "hihat_patch_keywords",
    ] {
        assert!(!table.contains_key(key), "トップレベルに {key} がある");
    }
}

#[test]
fn default_config_content_shows_how_to_configure_floe_presets() {
    let content = default_config_content();

    assert!(content.contains("# [plugins.Floe]"), "{content}");
    assert!(content.contains("Floe\\presets"), "{content}");
    assert!(content.contains(".floe-preset"), "{content}");
}

#[test]
fn default_config_content_uses_empty_loop_dirs() {
    let content = default_config_content();

    assert!(content.contains("loop_dirs = []"));
    assert!(!content.contains(r"N:\app4HDD\MAGIX"));
}

#[test]
fn default_config_content_uses_loop_categories() {
    let content = default_config_content();

    assert!(content
        .contains("loop_categories = [\"guitar\", \"drum\", \"bass\", \"spoken\", \"sequence\"]"));
}

#[test]
fn default_config_content_uses_voicing_source_urls() {
    let content = default_config_content();

    assert!(content.contains(&format!(
        "voicing_shared_source = \"{DEFAULT_VOICING_SHARED_SOURCE}\""
    )));
    assert!(content.contains(&format!(
        "voicing_override_source = \"{DEFAULT_VOICING_OVERRIDE_SOURCE}\""
    )));
    assert!(content.contains(&format!(
        "chord_progression_source = \"{DEFAULT_CHORD_PROGRESSION_SOURCE}\""
    )));
}

/// 旧用途別 7 項目は新しい catalog PatchRole へ置き換えたため、コメントにも残さない。
#[test]
fn default_config_content_does_not_mention_retired_patch_role_settings() {
    let content = default_config_content();

    for key in [
        "chord_patch_categories",
        "bass_patch_categories",
        "arpeggio_patch_categories",
        "drum_patch_categories",
        "kick_patch_keywords",
        "snare_patch_keywords",
        "hihat_patch_keywords",
    ] {
        assert!(
            !content.contains(key),
            "廃止した用途別設定 {key} をひな形へ残してはいけない"
        );
    }
}

#[test]
fn default_config_content_keeps_the_surge_profile_example() {
    let content = default_config_content();

    assert!(content.contains(r#"# [plugins."Surge XT"]"#), "{content}");
    assert!(content.contains("# plugin_path  = 'D:\\my\\clap\\Surge XT.clap'"));
    assert!(content.contains("# patches_dirs = ['D:\\my\\patches']"));
}

#[test]
fn default_config_content_uses_config_editor_key() {
    let content = default_config_content();

    assert!(
        content.contains(r#"editors = ["fresh", "zed", "code", "edit", "nano", "vim"]"#),
        "default config は editors を案内するべき: {}",
        content
    );
}

#[test]
fn default_config_content_uses_offline_render_workers_key() {
    let content = default_config_content();

    assert!(
        content.contains("offline_render_workers = 2"),
        "default config は offline_render_workers を案内するべき: {}",
        content
    );
    assert!(
        content.contains("offline_render_server_workers = 4"),
        "default config は offline_render_server_workers を案内するべき: {}",
        content
    );
}

#[test]
fn default_config_content_uses_offline_render_backend_keys() {
    let content = default_config_content();

    assert!(
        content.contains("offline_render_backend = \"in_process\""),
        "default config は backend 既定値を案内するべき: {}",
        content
    );
    assert!(
        content.contains("offline_render_server_port = 62153"),
        "default config は render-server port を案内するべき: {}",
        content
    );
    assert!(
        content.contains("offline_render_server_command = \"\""),
        "default config は render-server command を案内するべき: {}",
        content
    );
    assert!(
        content.contains("realtime_audio_backend = \"cache_player\""),
        "default config は realtime audio backend を案内するべき: {}",
        content
    );
    assert!(
        content.contains("realtime_play_server_port = 62154"),
        "default config は realtime play server port を案内するべき: {}",
        content
    );
    assert!(
        content.contains("realtime_play_server_command = \"\""),
        "default config は realtime play server command を案内するべき: {}",
        content
    );
}

#[test]
fn default_config_content_omits_removed_patch_path_key() {
    let content = default_config_content();

    assert!(
        !content.contains("patch_path"),
        "default config に削除済みの patch_path を残すべきではない: {}",
        content
    );
}

#[test]
fn default_config_content_preserves_windows_path_format() {
    let content = default_config_content();

    assert!(
        content.contains(
            r"# 例 (Windows): patches_dirs = ['C:\ProgramData\Surge XT\patches_factory', 'C:\ProgramData\Surge XT\patches_3rdparty']"
        ),
        "Windows の例示パスは単一バックスラッシュ表記を維持するべき: {}",
        content
    );
}

#[test]
fn default_config_content_omits_removed_daw_size_keys() {
    let content = default_config_content();

    assert!(
        !content.contains("daw_tracks"),
        "default config に削除済みの daw_tracks を残すべきではない: {}",
        content
    );
    assert!(
        !content.contains("daw_measures"),
        "default config に削除済みの daw_measures を残すべきではない: {}",
        content
    );
}
