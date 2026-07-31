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

#[test]
fn default_config_content_lists_the_chord_patch_categories() {
    let content = default_config_content();

    assert!(
        content.contains(r#"chord_patch_categories = ["Keys", "Organs", "Pads", "Polysynths"]"#),
        "{content}"
    );
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
        content.contains("realtime_audio_backend = \"in_process\""),
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
