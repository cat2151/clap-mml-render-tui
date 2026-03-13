mod config;
mod host;
mod midi;
mod patch_list;
mod pipeline;
mod render;
mod tui;

use anyhow::Result;

fn main() -> Result<()> {
    let cfg = config::Config::load()?;

    // CLAP プラグインエントリをロード（TUI/CLI 共通）
    let entry = host::load_entry(&cfg.plugin_path)?;

    // 引数なし → TUI モード
    // --mml "cde" → CLI パイプラインモード（テスト用）
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--mml") {
        if let Some(mml) = args.get(pos + 1) {
            println!("CLI モード: MML = {}", mml);
            let patch = pipeline::mml_to_play(mml, &cfg, &entry)?;
            println!("patch: {}", patch);
            return Ok(());
        }
    }

    // TUI モード
    let mut app = tui::TuiApp::new(&cfg, &entry);
    app.run()?;

    Ok(())
}

