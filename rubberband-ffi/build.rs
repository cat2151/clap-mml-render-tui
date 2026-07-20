use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const REPOSITORY: &str = "https://github.com/breakfastquay/rubberband";
const UPSTREAM_BRANCH: &str = "default";
const UPSTREAM_REVISION: &str = "e4296ac80b1170018a110bc326fd0d45a0eb27d6";
const SUPPORTED_C_API_MAJOR: &str = "3";

fn main() {
    if let Err(error) = run() {
        panic!("Rubber Band の取得またはビルドに失敗しました: {error}");
    }
}

fn run() -> Result<(), String> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR がありません")?);
    let source_dir = out_dir.join("rubberband-src");
    prepare_source(&source_dir)?;
    validate_c_api(&source_dir)?;
    let revision = git_output(&source_dir, &["rev-parse", "HEAD"])?;

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file(source_dir.join("single/RubberBandSingle.cpp"))
        .include(&source_dir)
        .define("RUBBERBAND_STATIC", None)
        .define("NOMINMAX", None)
        .flag_if_supported("-std=c++11")
        .flag_if_supported("/std:c++14")
        .warnings(false)
        .compile("rubberband");

    if env::var("CARGO_CFG_TARGET_VENDOR").as_deref() == Ok("apple") {
        println!("cargo:rustc-link-lib=framework=Accelerate");
    }
    println!("cargo:rustc-env=RUBBERBAND_GIT_REV={revision}");
    println!("cargo:rerun-if-env-changed=RUBBERBAND_CXXFLAGS");
    Ok(())
}

fn prepare_source(source_dir: &Path) -> Result<(), String> {
    if source_dir.join(".git").is_dir() {
        return checkout_pinned_revision(source_dir);
    }

    if source_dir.exists() {
        let out_dir = source_dir
            .parent()
            .ok_or("Rubber Band source の親ディレクトリがありません")?;
        if !source_dir.starts_with(out_dir) {
            return Err("Rubber Band source path が OUT_DIR 外です".to_string());
        }
        fs::remove_dir_all(source_dir)
            .map_err(|error| format!("不完全な取得先を削除できません: {error}"))?;
    }

    let parent = source_dir
        .parent()
        .ok_or("Rubber Band source の親ディレクトリがありません")?;
    let name = source_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Rubber Band source のディレクトリ名が不正です")?;
    let output = Command::new("git")
        .current_dir(parent)
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            UPSTREAM_BRANCH,
            REPOSITORY,
            name,
        ])
        .output()
        .map_err(|error| format!("git clone を起動できません: {error}"))?;
    ensure_success("git clone", output)?;
    checkout_pinned_revision(source_dir)
}

fn checkout_pinned_revision(source_dir: &Path) -> Result<(), String> {
    let current_revision = git_output(source_dir, &["rev-parse", "HEAD"])?;
    if current_revision != UPSTREAM_REVISION {
        run_git(
            source_dir,
            &["fetch", "--depth", "1", "origin", UPSTREAM_REVISION],
        )?;
    }
    run_git(source_dir, &["reset", "--hard", UPSTREAM_REVISION])?;

    let checked_out_revision = git_output(source_dir, &["rev-parse", "HEAD"])?;
    if checked_out_revision != UPSTREAM_REVISION {
        return Err(format!(
            "Rubber Band revision が一致しません（expected {UPSTREAM_REVISION}, actual {checked_out_revision}）"
        ));
    }
    Ok(())
}

fn validate_c_api(source_dir: &Path) -> Result<(), String> {
    let header = source_dir.join("rubberband/rubberband-c.h");
    let text = fs::read_to_string(&header)
        .map_err(|error| format!("{} を読めません: {error}", header.display()))?;
    let expected = format!("#define RUBBERBAND_API_MAJOR_VERSION {SUPPORTED_C_API_MAJOR}");
    if !text.contains(&expected) {
        return Err(format!(
            "未対応の Rubber Band C API です（expected major {SUPPORTED_C_API_MAJOR}）"
        ));
    }
    Ok(())
}

fn run_git(dir: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|error| format!("git {} を起動できません: {error}", args.join(" ")))?;
    ensure_success(&format!("git {}", args.join(" ")), output)
}

fn git_output(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|error| format!("git {} を起動できません: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(command_error(&format!("git {}", args.join(" ")), &output));
    }
    String::from_utf8(output.stdout)
        .map(|text| text.trim().to_string())
        .map_err(|error| format!("git output がUTF-8ではありません: {error}"))
}

fn ensure_success(label: &str, output: Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(command_error(label, &output))
    }
}

fn command_error(label: &str, output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!(
        "{label} が終了コード {} で失敗しました: {stderr}",
        output.status
    )
}
