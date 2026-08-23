pub(crate) fn version_text() -> String {
    format!(
        "{} (git {}, Rubber Band C API {} @ {})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_COMMIT_HASH"),
        rubberband_ffi::C_API_MAJOR_VERSION,
        rubberband_ffi::GIT_REVISION
    )
}

pub(crate) fn print_help(help: &str) {
    print!("{}", help);
    if !help.ends_with('\n') {
        println!();
    }
    println!();
    match clap_mml_render_tui::config::config_file_path() {
        Some(path) => println!("設定ファイル: {}", path.display()),
        None => println!("設定ファイル: (システムの設定ディレクトリが見つかりません)"),
    }
}
