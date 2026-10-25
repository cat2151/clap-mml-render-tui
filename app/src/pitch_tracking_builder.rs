//! `cmrt build-pitch-tracking`: drum の percussion 行に入る音色を全部鳴らし、
//! note を変えると音高がついてくるかを 1 ファイルの表に書く。
//!
//! 対象選びはここ、1 音色の計測は play server の `probe-pitch-tracking`。
//! 1 音色ごとに server を子プロセスで起動するので、plugin が落ちても一括処理は止まらない。

use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use cmrt_patches::{DrumPatchRole, PatchRoleIndex, PatchRoleInput};
use cmrt_realtime_play::{server_binary::resolve_server_binary, ServerBinary};
use serde::{Deserialize, Serialize};

use crate::config::Config;

pub const FORMAT_VERSION: u32 = 1;

/// 1 音色の計測がこれを超えたら server を落として失敗扱いにする。
///
/// sample を読む sfz は 1 音色で十数秒かかるので、それより十分長くとる。
const PROBE_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPitchTrackingRequest {
    pub output: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PitchTrackingFile {
    pub format_version: u32,
    pub entries: Vec<PitchTrackingEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PitchTrackingEntry {
    pub plugin_id: String,
    pub display: String,
    pub tracks_keyboard: bool,
    pub observations: Vec<NoteFrequency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteFrequency {
    pub note: u8,
    pub frequency_hz: Option<f64>,
}

/// `probe-pitch-tracking --json` の出力。
#[derive(Debug, Deserialize, PartialEq)]
struct ProbeReport {
    tracks_keyboard: bool,
    observations: Vec<NoteFrequency>,
}

/// 対象選びに使う、catalog の 1 音色。
#[derive(Clone, Copy)]
pub(crate) struct TargetInput<'a> {
    pub(crate) display: &'a str,
    pub(crate) normalized_display: &'a str,
    pub(crate) selector_category: Option<&'a str>,
    pub(crate) plugin_name: Option<&'a str>,
    pub(crate) plugin_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) display: String,
    /// catalog の plugin に id が無ければ `None`。計測せず失敗として数える。
    pub(crate) plugin_id: Option<String>,
}

/// 全 plugin の音色をまとめて分類し、drum の percussion 行に入るものだけを catalog 順で返す。
///
/// ユーザー定義の Role preset は渡さない（cat-plugin-player も渡さずに分類するので、揃える）。
pub(crate) fn percussion_targets(inputs: &[TargetInput<'_>]) -> Vec<Target> {
    let roles = PatchRoleIndex::build(
        inputs.iter().map(|input| PatchRoleInput {
            display: input.display,
            normalized_display: input.normalized_display,
            selector_category: input.selector_category,
            plugin: input.plugin_name,
        }),
        &[],
    );
    inputs
        .iter()
        .filter(|input| roles.drum_role_of(input.display) == Some(DrumPatchRole::Percussion))
        .map(|input| Target {
            display: input.display.to_string(),
            plugin_id: input.plugin_id.map(str::to_string),
        })
        .collect()
}

/// server の stdout から、最後に出た JSON を読む。
///
/// plugin によっては stdout へ初期化ログを書く（Six Sines など）。JSON は server が最後に
/// 整形して出すので、行頭から `{` だけの行（字下げした内側の object は除く）のうち最後のものから読む。
fn parse_probe_stdout(stdout: &str) -> Result<ProbeReport> {
    let start = stdout
        .match_indices('\n')
        .map(|(index, _)| index + 1)
        .chain(std::iter::once(0))
        .filter(|&start| {
            stdout[start..]
                .lines()
                .next()
                .is_some_and(|line| line.trim_end() == "{")
        })
        .max()
        .context("stdout に JSON がありません")?;
    serde_json::from_str(&stdout[start..]).context("stdout の JSON を読めません")
}

/// 1 音色の計測が失敗した理由。集計で種類ごとに数える。
#[derive(Debug)]
enum ProbeFailure {
    NoPluginId,
    Spawn(String),
    Timeout,
    Exit { code: Option<i32>, stderr: String },
    Output(String),
}

impl ProbeFailure {
    fn kind(&self) -> &'static str {
        match self {
            ProbeFailure::NoPluginId => "plugin_id 不明",
            ProbeFailure::Spawn(_) => "起動失敗",
            ProbeFailure::Timeout => "タイムアウト",
            ProbeFailure::Exit { .. } => "異常終了",
            ProbeFailure::Output(_) => "出力不正",
        }
    }
}

impl std::fmt::Display for ProbeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeFailure::NoPluginId => write!(f, "catalog の plugin に id がありません"),
            ProbeFailure::Spawn(error) => write!(f, "server を起動できません: {error}"),
            ProbeFailure::Timeout => {
                write!(f, "{}s 以内に終わりませんでした", PROBE_TIMEOUT.as_secs())
            }
            ProbeFailure::Exit { code, stderr } => {
                write!(f, "exit={code:?} stderr={stderr}")
            }
            ProbeFailure::Output(error) => write!(f, "{error}"),
        }
    }
}

fn probe(server_exe: &Path, display: &str) -> Result<ProbeReport, ProbeFailure> {
    let mut child = Command::new(server_exe)
        .args(["probe-pitch-tracking", "--patch", display, "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ProbeFailure::Spawn(error.to_string()))?;
    // pipe が詰まって子が止まらないよう、待つ間も別スレッドで読み続ける。
    let stdout = read_in_background(child.stdout.take());
    let stderr = read_in_background(child.stderr.take());
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > PROBE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProbeFailure::Timeout);
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(ProbeFailure::Spawn(error.to_string())),
        }
    };
    let stdout = stdout.join().unwrap_or_default();
    let stderr = stderr.join().unwrap_or_default();
    if !status.success() {
        return Err(ProbeFailure::Exit {
            code: status.code(),
            stderr: last_line(&stderr).to_string(),
        });
    }
    parse_probe_stdout(&stdout).map_err(|error| ProbeFailure::Output(format!("{error:#}")))
}

fn read_in_background<R: Read + Send + 'static>(pipe: Option<R>) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

fn last_line(text: &str) -> &str {
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim()
}

#[derive(Default)]
struct PluginTally {
    targets: usize,
    tracks_keyboard: usize,
    failed: usize,
}

pub fn run(cfg: &Config, request: &BuildPitchTrackingRequest) -> Result<()> {
    let started = Instant::now();
    let server_exe = match resolve_server_binary(cfg.play_server_launch_override.as_ref()) {
        ServerBinary::Resolved(resolved) => {
            println!("play server: {}", resolved.log_fields());
            PathBuf::from(resolved.exe)
        }
        ServerBinary::NotFound { searched } => {
            anyhow::bail!("{}", ServerBinary::not_found_lines(&searched).join("\n"))
        }
    };
    let (snapshot, _) = crate::patch_catalog_cache::load()
        .with_context(|| {
            format!(
                "patch catalog cache を読めません。先に `{}` を実行してください",
                crate::patch_catalog_cache::BUILD_COMMAND
            )
        })?
        .into_parts();
    let plugins = snapshot.patch_plugins();
    let infos = snapshot
        .audio_patches()
        .iter()
        .map(|patch| plugins.audio_info_for_ref(&patch.reference).ok())
        .collect::<Vec<_>>();
    let inputs = snapshot
        .audio_patches()
        .iter()
        .zip(&infos)
        .map(|(patch, info)| TargetInput {
            display: &patch.reference.display,
            normalized_display: &patch.normalized_display,
            selector_category: patch.selector_category.as_deref(),
            plugin_name: info.map(|info| info.name.as_str()),
            plugin_id: info.and_then(|info| info.plugin_id.as_deref()),
        })
        .collect::<Vec<_>>();
    let plugin_names = inputs
        .iter()
        .filter_map(|input| Some((input.display, input.plugin_name?)))
        .collect::<BTreeMap<_, _>>();
    let targets = percussion_targets(&inputs);
    println!(
        "catalog {} 件のうち drum の percussion 行は {} 件。1 件ずつ計測します。",
        inputs.len(),
        targets.len()
    );

    let total = targets.len();
    let mut entries = Vec::with_capacity(total);
    let mut tallies = BTreeMap::<&str, PluginTally>::new();
    let mut failures = BTreeMap::<&'static str, (usize, String)>::new();
    for (index, target) in targets.iter().enumerate() {
        let plugin = plugin_names
            .get(target.display.as_str())
            .copied()
            .unwrap_or("(不明)");
        let tally = tallies.entry(plugin).or_default();
        tally.targets += 1;
        print!("[{}/{}] {plugin} {} -> ", index + 1, total, target.display);
        let item_started = Instant::now();
        let result = match &target.plugin_id {
            Some(plugin_id) => {
                probe(&server_exe, &target.display).map(|report| (plugin_id, report))
            }
            None => Err(ProbeFailure::NoPluginId),
        };
        let seconds = item_started.elapsed().as_secs_f64();
        match result {
            Ok((plugin_id, report)) => {
                println!("tracks_keyboard={} ({seconds:.1}s)", report.tracks_keyboard);
                if report.tracks_keyboard {
                    tally.tracks_keyboard += 1;
                }
                entries.push(PitchTrackingEntry {
                    plugin_id: plugin_id.clone(),
                    display: target.display.clone(),
                    tracks_keyboard: report.tracks_keyboard,
                    observations: report.observations,
                });
            }
            Err(failure) => {
                // 1 件の失敗で全体を止めない。種類ごとに数え、代表 1 件を残す。
                println!("失敗[{}] {failure} ({seconds:.1}s)", failure.kind());
                tally.failed += 1;
                failures
                    .entry(failure.kind())
                    .or_insert_with(|| (0, format!("{plugin} {}: {failure}", target.display)))
                    .0 += 1;
            }
        }
    }

    write_file(
        &request.output,
        &PitchTrackingFile {
            format_version: FORMAT_VERSION,
            entries,
        },
    )?;
    println!();
    println!("[plugin ごとの件数] 対象 / tracks_keyboard=true / 失敗");
    for (plugin, tally) in &tallies {
        println!(
            "  {plugin}: {} / {} / {}",
            tally.targets, tally.tracks_keyboard, tally.failed
        );
    }
    println!("[失敗の種類] 件数: 代表");
    if failures.is_empty() {
        println!("  なし");
    }
    for (kind, (count, example)) in &failures {
        println!("  {kind} {count} 件: {example}");
    }
    println!(
        "書き出しました: {} （所要 {:.0}s）",
        request.output.display(),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

/// 一時ファイルへ書いてから置き換える。途中で落ちても前の表が壊れないように。
fn write_file(path: &Path, file: &PitchTrackingFile) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(file)?;
    bytes.push(b'\n');
    let mut temp_name = path.as_os_str().to_owned();
    temp_name.push(format!(".{}.tmp", std::process::id()));
    let temp_path = PathBuf::from(temp_name);
    std::fs::write(&temp_path, bytes)
        .with_context(|| format!("一時ファイルを書けません: {}", temp_path.display()))?;
    if let Err(error) = std::fs::rename(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error).with_context(|| format!("置き換えられません: {}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
