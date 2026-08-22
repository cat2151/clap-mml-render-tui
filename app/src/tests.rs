use super::*;

#[test]
fn version_reports_rubber_band_api_and_revision() {
    let CliAction::Version(version) = parse_cli_from(["cmrt", "--version"]).unwrap() else {
        panic!("--version should return version text");
    };
    assert!(version.contains("Rubber Band C API 3"));
    assert!(version.contains(rubberband_ffi::GIT_REVISION));
}

#[test]
fn update_subcommand_is_recognized() {
    assert_eq!(
        parse_cli_from(["cmrt", "update"]).unwrap(),
        CliAction::Update
    );
}

#[test]
fn update_subcommand_takes_precedence_over_cli_mml_mode() {
    assert_ne!(
        parse_cli_from(["cmrt", "update"]).unwrap(),
        CliAction::CliMml("update".to_string())
    );
}

#[test]
fn check_subcommand_is_recognized() {
    assert_eq!(parse_cli_from(["cmrt", "check"]).unwrap(), CliAction::Check);
}

#[test]
fn scan_loops_subcommand_is_recognized() {
    assert_eq!(
        parse_cli_from(["cmrt", "scan-loops"]).unwrap(),
        CliAction::ScanLoops
    );
}

#[test]
fn build_voicing_cache_subcommand_is_recognized() {
    assert_eq!(
        parse_cli_from(["cmrt", "build-voicing-cache"]).unwrap(),
        CliAction::BuildVoicingCache { force: false }
    );
    assert_eq!(
        parse_cli_from(["cmrt", "build-voicing-cache", "--force"]).unwrap(),
        CliAction::BuildVoicingCache { force: true }
    );
}

/// 診断の `--config` は省略でき、渡せばそのパスがそのまま届く。
/// 実ユーザーの config.toml を書き換えて戻す運用（戻し忘れが即事故）を無くすための入口。
#[test]
fn patch_roles_subcommand_takes_an_optional_config_path() {
    assert_eq!(
        parse_cli_from(["cmrt", "patch-roles"]).unwrap(),
        CliAction::PatchRoles { config: None }
    );
    assert_eq!(
        parse_cli_from(["cmrt", "patch-roles", "--config", "/tmp/try.toml"]).unwrap(),
        CliAction::PatchRoles {
            config: Some(PathBuf::from("/tmp/try.toml"))
        }
    );
}

/// `render-mml` は音色も MML も省略でき、複数の `--patch` を並べられる。
#[test]
fn render_mml_subcommand_collects_every_patch_option() {
    assert_eq!(
        parse_cli_from(["cmrt", "render-mml"]).unwrap(),
        CliAction::RenderMml(RenderMmlRequest::default())
    );
    assert_eq!(
        parse_cli_from([
            "cmrt",
            "render-mml",
            "--config",
            "/tmp/try.toml",
            "--patch",
            "AR Accent Arp.vvp",
            "--patch",
            "Pads/Pad 1.fxp",
            "--out-dir",
            "/tmp/wav",
            "--poly-check",
            "cde",
        ])
        .unwrap(),
        CliAction::RenderMml(RenderMmlRequest {
            config: Some(PathBuf::from("/tmp/try.toml")),
            patches: vec![
                "AR Accent Arp.vvp".to_string(),
                "Pads/Pad 1.fxp".to_string()
            ],
            mml: Some("cde".to_string()),
            out_dir: Some(PathBuf::from("/tmp/wav")),
            poly_check: true,
        })
    );
}

/// `render-mml` という文字列が MML として再生されないこと。
#[test]
fn render_mml_subcommand_takes_precedence_over_cli_mml_mode() {
    assert_ne!(
        parse_cli_from(["cmrt", "render-mml"]).unwrap(),
        CliAction::CliMml("render-mml".to_string())
    );
}

#[test]
fn check_subcommand_takes_precedence_over_cli_mml_mode() {
    assert_ne!(
        parse_cli_from(["cmrt", "check"]).unwrap(),
        CliAction::CliMml("check".to_string())
    );
}

#[test]
fn cli_mml_mode_still_accepts_regular_positional_argument() {
    assert_eq!(
        parse_cli_from(["cmrt", "cde"]).unwrap(),
        CliAction::CliMml("cde".to_string())
    );
}

#[test]
fn cli_playback_mml_converts_single_chord_to_mml() {
    assert_eq!(
        cli_playback_mml("C"),
        CliPlaybackMml::Chord {
            chord: "C".to_string(),
            mml: "v11'c1eg'".to_string(),
        }
    );
}

#[test]
fn cli_playback_mml_converts_chord_progression_to_mml() {
    assert_eq!(
        cli_playback_mml("Dm G7 C"),
        CliPlaybackMml::Chord {
            chord: "Dm G7 C".to_string(),
            mml: "v11'd1fa''g1b<df''c1eg'".to_string(),
        }
    );
}

#[test]
fn cli_playback_mml_keeps_regular_mml_when_chord_parse_fails() {
    assert_eq!(
        cli_playback_mml("cde"),
        CliPlaybackMml::Mml("cde".to_string())
    );
}

#[test]
fn server_flag_uses_default_port_when_value_is_omitted() {
    assert_eq!(
        parse_cli_from(["cmrt", "--server"]).unwrap(),
        CliAction::Server(server::DEFAULT_PORT)
    );
}

#[test]
fn shutdown_flag_uses_default_port_when_value_is_omitted() {
    assert_eq!(
        parse_cli_from(["cmrt", "--shutdown"]).unwrap(),
        CliAction::Shutdown(server::DEFAULT_PORT)
    );
}

#[test]
fn deprecated_mml_flag_returns_guidance() {
    let err = parse_cli_from(["cmrt", "--mml", "cde"]).unwrap_err();

    assert!(err
        .to_string()
        .contains("`--mml` オプションは廃止されました。`cmrt <mml>` の形式で指定してください。"));
}

#[test]
fn deprecated_mml_flag_without_value_returns_same_guidance() {
    let err = parse_cli_from(["cmrt", "--mml"]).unwrap_err();

    assert!(err
        .to_string()
        .contains("`--mml` オプションは廃止されました。`cmrt <mml>` の形式で指定してください。"));
}

#[test]
fn top_level_help_uses_runtime_default_port() {
    match parse_cli_from(["cmrt", "--help"]).unwrap() {
        CliAction::Help(help) => assert!(help.contains(&format!(
            "curl -X POST http://127.0.0.1:{}/ --data 'cde'",
            server::DEFAULT_PORT
        ))),
        other => panic!("expected help action, got {other:?}"),
    }
}

#[test]
fn subcommand_help_is_preserved() {
    match parse_cli_from(["cmrt", "update", "--help"]).unwrap() {
        CliAction::Help(help) => {
            assert!(help.contains("Usage: cmrt update"));
            assert!(help.contains("アップデートを実行"));
        }
        other => panic!("expected help action, got {other:?}"),
    }
}

#[test]
fn check_subcommand_help_is_preserved() {
    match parse_cli_from(["cmrt", "check", "--help"]).unwrap() {
        CliAction::Help(help) => {
            assert!(help.contains("Usage: cmrt check"));
            assert!(help.contains("ビルド時コミットと remote main を比較"));
        }
        other => panic!("expected help action, got {other:?}"),
    }
}
