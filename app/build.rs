fn main() {
    // ビルド時のgitコミットhashを埋め込む
    let hash = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", hash);

    // .git/HEAD 自体の変更を監視
    let head_path = "../.git/HEAD";
    println!("cargo:rerun-if-changed={}", head_path);

    // HEAD がシンボリックリファレンス (例: "ref: refs/heads/main") の場合は、
    // 参照先の refs ファイルおよび packed-refs も監視対象に含める。
    if let Ok(head_contents) = std::fs::read_to_string(head_path) {
        if let Some(rest) = head_contents.strip_prefix("ref:") {
            let ref_path = rest.trim();
            if !ref_path.is_empty() {
                println!("cargo:rerun-if-changed=../.git/{}", ref_path);
            }
        }
        // 個々のファイルではなく packed-refs に格納される場合もある
        println!("cargo:rerun-if-changed=../.git/packed-refs");
    }

    // RubberBandLiveShifter と tree-sitter grammar が dllexport 付きでリンクされるため、
    // MSVC link.exe が exe 用の .lib/.exp を作って linker_messages 警告を出す。exe に不要なので作らせない。
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/NOIMPLIB");
        println!("cargo:rustc-link-arg-bins=/NOEXP");
    }
}
