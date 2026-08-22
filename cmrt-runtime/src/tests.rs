use super::*;

#[test]
fn config_parse_uses_runtime_defaults() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
"#;

    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert_eq!(cfg.offline_render_workers, DEFAULT_OFFLINE_RENDER_WORKERS);
    assert_eq!(
        cfg.offline_render_server_workers,
        DEFAULT_OFFLINE_RENDER_SERVER_WORKERS
    );
    assert_eq!(cfg.offline_render_backend, OfflineRenderBackend::InProcess);
    assert_eq!(
        cfg.offline_render_server_port,
        DEFAULT_OFFLINE_RENDER_SERVER_PORT
    );
    assert!(cfg.offline_render_server_command.is_empty());
    assert_eq!(cfg.realtime_audio_backend, RealtimeAudioBackend::InProcess);
    assert_eq!(
        cfg.realtime_play_server_port,
        DEFAULT_REALTIME_PLAY_SERVER_PORT
    );
    assert!(cfg.realtime_play_server_command.is_empty());
    assert!(cfg.autoplay_on_startup);
    assert!(cfg.loop_dirs.is_empty());
    assert_eq!(cfg.loop_categories, default_loop_categories());
}

#[test]
fn config_autoplay_on_startup_parses_explicit_false() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
autoplay_on_startup = false
"#;

    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert!(!cfg.autoplay_on_startup);
}

#[test]
fn default_config_content_contains_render_server_keys() {
    let content = default_config_content();

    assert!(content.contains("offline_render_workers = 2"));
    assert!(content.contains("offline_render_backend = \"in_process\""));
    assert!(content.contains("offline_render_server_workers = 4"));
    assert!(content.contains("offline_render_server_port = 62153"));
    assert!(content.contains("offline_render_server_command = \"\""));
    assert!(content.contains("realtime_audio_backend = \"in_process\""));
    assert!(content.contains("realtime_play_server_port = 62154"));
    assert!(content.contains("realtime_play_server_command = \"\""));
    assert!(content.contains("autoplay_on_startup = true"));
    assert!(content.contains("loop_dirs = []"));
    assert!(content
        .contains("loop_categories = [\"guitar\", \"drum\", \"bass\", \"spoken\", \"sequence\"]"));
}

#[test]
fn config_loop_dirs_parse_and_validate() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
loop_dirs = ["/loops/one", "/loops/two"]
"#;

    let cfg: Config = toml::from_str(toml_str).unwrap();
    cfg.validate().unwrap();
    assert_eq!(cfg.loop_dirs, ["/loops/one", "/loops/two"]);
}

#[test]
fn config_loop_dirs_reject_empty_entry() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
loop_dirs = [""]
"#;

    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert!(cfg.validate().is_err());
}

#[test]
fn config_loop_categories_parse_and_validate() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
loop_categories = ["guitar", "drum", "bass"]
"#;

    let cfg: Config = toml::from_str(toml_str).unwrap();
    cfg.validate().unwrap();
    assert_eq!(cfg.loop_categories, ["guitar", "drum", "bass"]);
}

#[test]
fn config_loop_categories_reject_invalid_entries() {
    for categories in [
        "[\"\"]".to_string(),
        "[\"bass\", \"bass\"]".to_string(),
        format!(
            "[{}]",
            (0..27)
                .map(|index| format!("\"category-{index}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ] {
        let toml_str = format!(
            r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
loop_categories = {categories}
"#
        );
        let cfg: Config = toml::from_str(&toml_str).unwrap();
        assert!(cfg.validate().is_err(), "accepted {categories}");
    }
}

#[test]
fn config_realtime_audio_backend_parses_play_server() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
realtime_audio_backend = "play_server"
realtime_play_server_port = 62154
realtime_play_server_command = "clap-mml-realtime-play-server"
"#;

    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert_eq!(cfg.realtime_audio_backend, RealtimeAudioBackend::PlayServer);
    assert_eq!(cfg.realtime_play_server_port, 62154);
    assert_eq!(
        cfg.realtime_play_server_command,
        "clap-mml-realtime-play-server"
    );
}

#[test]
fn config_realtime_play_server_port_validation_rejects_zero() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
realtime_play_server_port = 0
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert!(cfg.validate().is_err());
}

#[test]
fn shared_patch_root_dir_returns_common_parent() {
    let dirs = vec![
        "/tmp/surge-data/patches_factory".to_string(),
        "/tmp/surge-data/patches_3rdparty".to_string(),
    ];

    let base = shared_patch_root_dir(&dirs);

    assert_eq!(base.as_deref(), Some("/tmp/surge-data"));
}

#[test]
fn core_config_patch_root_dir_uses_shared_patch_root() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
patches_dirs = ["/tmp/surge-data/patches_factory", "/tmp/surge-data/patches_3rdparty"]
"#;

    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert_eq!(
        core_config_patch_root_dir(&cfg).as_deref(),
        Some("/tmp/surge-data")
    );
}

/// Vaporizer2 は**音色置き場の既定値を持たない**（プリセットの置き場所がインストールごとに
/// 違う）ので、patches_dirs を書く場所をひな形が案内していないと、インストール済みでも
/// 音色が 1 件も一覧に出ない。
#[test]
fn the_default_config_shows_a_commented_vaporizer2_profile() {
    let content = default_config_content();

    assert!(content.contains("# [plugins.Vaporizer2]"), "{content}");
    assert!(content.contains("# patches_dirs = "), "{content}");
    assert!(
        content.contains(
            r#"# chord_patch_categories = ["Pad", "Chord", "Organ", "Synth", "Atmosphere"]"#
        ),
        "{content}"
    );
    assert!(
        content.contains(r#"# bass_patch_categories = ["Bass"]"#),
        "{content}"
    );
    // active_plugin の案内にも 3 つめとして出す。
    assert!(content.contains("'Vaporizer2'"), "{content}");
}

/// カテゴリは `.vvp` のファイル名先頭 2 文字なので、**対応表が無いとユーザーは自分の
/// 音色置き場を見ても何を書けばよいか分からない**。表は [`cmrt_patches::vaporizer2`] が
/// 単一ソースなので、そこに足したコードがひな形にも必ず出ることを見る。
#[test]
fn the_default_config_lists_every_vaporizer2_category_code() {
    let content = default_config_content();

    for (code, name) in cmrt_patches::vaporizer2::PATCH_CATEGORY_CODES {
        assert!(
            content.contains(&format!("{code} {name}")),
            "カテゴリコード {code} の案内がひな形に無い"
        );
    }
}

/// ひな形はそのままでも、末尾のコメント済みプロファイルを丸ごと有効にしても TOML として
/// 通る。テーブル見出しが末尾にあることの担保（途中に置くと、後続のトップレベル項目が
/// `[plugins."Surge XT"]` の中身になって型が合わなくなる）。
#[test]
fn the_default_config_parses_with_and_without_the_commented_profile() {
    let content = default_config_content();
    let cfg: Config = toml::from_str(&content).expect("ひな形がそのまま TOML として通ること");
    // 用途別 7 項目はトップレベルに書かれていない。既定値はプラグインごとに持つ。
    assert_eq!(cfg.top_level_patch_roles, PatchRoleFilters::default());

    let header = content
        .rfind(r#"# [plugins."Surge XT"]"#)
        .expect("コメント済みプロファイル");
    // ユーザーがやるのと同じで、設定行（`# キー = 値` と `# [テーブル]`）だけコメントを外す。
    // 説明文にも `=` は出るので、`=` の左が TOML のキーの形をしている行だけを設定行とみなす。
    let is_setting = |body: &str| {
        body.starts_with('[')
            || body.split_once('=').is_some_and(|(key, _)| {
                let key = key.trim();
                !key.is_empty() && key.chars().all(|c| c.is_ascii_lowercase() || c == '_')
            })
    };
    let uncommented: String = content[header..]
        .lines()
        .map(|line| match line.strip_prefix("# ") {
            Some(body) if is_setting(body) => body.trim_end(),
            _ => line.trim_end(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let with_profile = format!("{}{uncommented}\n", &content[..header]);
    let cfg: Config =
        toml::from_str(&with_profile).expect("コメントを外しても TOML として通ること");

    let surge = cfg.plugins.get("Surge XT").expect("Surge XT プロファイル");
    assert_eq!(
        surge.patch_roles.chord_patch_categories,
        Some(PatchRoles::builtin_for(Some(SURGE_XT_PLUGIN_ID), "").chord_patch_categories)
    );
    assert!(cfg.plugins.contains_key("my_synth"));
}
