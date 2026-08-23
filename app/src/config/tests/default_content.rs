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

/// 用途別 7 項目は**トップレベルへ値として書かない**。トップレベルの値は既定プラグインに
/// だけ効くレガシー綴りなので、Surge のカテゴリ名を書き出すと `active_plugin` に別の
/// プラグインを指した config で候補が全滅する（`docs/adr/0007-patch-role-defaults-three-layers.md`）。
#[test]
fn default_config_content_does_not_write_the_patch_roles_at_the_top_level() {
    let content = default_config_content();

    for line in content.lines() {
        assert!(
            !line.starts_with("chord_patch_categories")
                && !line.starts_with("bass_patch_categories")
                && !line.starts_with("arpeggio_patch_categories")
                && !line.starts_with("drum_patch_categories")
                && !line.starts_with("kick_patch_keywords")
                && !line.starts_with("snare_patch_keywords")
                && !line.starts_with("hihat_patch_keywords"),
            "用途別 7 項目をトップレベルへ書いてはいけない: {line}"
        );
    }
}

/// 値は見えないと編集できないので、`[plugins."Surge XT"]` のコメントとして案内する。
#[test]
fn default_config_content_shows_the_surge_patch_roles_as_a_commented_profile() {
    let content = default_config_content();

    assert!(content.contains(r#"# [plugins."Surge XT"]"#), "{content}");
    assert!(
        content.contains(r#"# chord_patch_categories = ["Keys", "Organs", "Pads", "Polysynths"]"#),
        "{content}"
    );
    assert!(
        content.contains(r#"# bass_patch_categories = ["Basses"]"#),
        "{content}"
    );
    assert!(
        content.contains(
            r#"# arpeggio_patch_categories = ["Bells", "Brass", "Guitars", "Keys", "Leads", "Mallets", "Modelled", "MPE", "Organs", "Plucks"]"#
        ),
        "{content}"
    );
}

/// コメントを外した瞬間に後続のトップレベル項目が吸い込まれないよう、テーブル見出しは
/// 必ずファイル末尾に置く。
#[test]
fn the_commented_profile_is_the_last_thing_in_the_default_config() {
    let content = default_config_content();
    let header = content
        .rfind(r#"# [plugins."Surge XT"]"#)
        .expect("commented profile header");

    for line in content[header..].lines() {
        let line = line.trim();
        assert!(
            line.is_empty() || line.starts_with('#'),
            "コメント済みプロファイルより後ろに設定行を置いてはいけない: {line}"
        );
    }
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
