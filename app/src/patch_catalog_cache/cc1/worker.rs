//! Keep native CLAP rendering outside the catalog builder. The worker keeps
//! one library open, while audio::observe creates a fresh instance per CC.

use std::{
    io::{self, BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use cmrt_core::{load_entry, AudioPluginInfo, PatchBase};
use cmrt_tui_core::patch_catalog_cache::cc1::Observation;
use serde::{Deserialize, Serialize};

use super::audio;

pub const CC1_WORKER_COMMAND: &str = "__cc1-audio-worker";
const REPLY_PREFIX: &str = "CMRT_CC1_REPLY ";
// The full drum-kit render took 53 s for both CC conditions on the installed catalog.
const PATCH_TIMEOUT: Duration = Duration::from_secs(90);
type Reply = Result<Observation, String>;

#[derive(Serialize, Deserialize)]
pub(super) struct Plugin {
    path: String,
    pub(super) id: String,
    pub(super) version: String,
    pub(super) patch_base: PatchBase,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Request {
    pub(super) patch_path: String,
    pub(super) routing_unresolved: String,
}

pub(super) struct Client {
    plugin: Plugin,
    worker: Option<Worker>,
}

impl Client {
    pub(super) fn new(info: &AudioPluginInfo, id: &str, version: &str) -> Self {
        Self {
            plugin: Plugin {
                path: info.plugin_path.clone(),
                id: id.into(),
                version: version.into(),
                patch_base: info.patch_base.clone(),
            },
            worker: None,
        }
    }

    pub(super) fn observe(
        &mut self,
        patch_path: String,
        routing_unresolved: String,
    ) -> Result<Observation> {
        if self.worker.is_none() {
            self.worker = Some(Worker::start(&self.plugin)?);
        }
        let request = Request {
            patch_path,
            routing_unresolved,
        };
        let result = self
            .worker
            .as_mut()
            .expect("worker was started")
            .measure(&request);
        match result {
            Ok(reply) => reply.map_err(anyhow::Error::msg),
            Err(error) => {
                // Drop kills and reaps the stalled worker. The next patch gets
                // a fresh process and cannot inherit the failed plugin state.
                self.worker = None;
                Err(error)
            }
        }
    }
}

struct Worker {
    child: Child,
    input: ChildStdin,
    replies: Receiver<serde_json::Result<Reply>>,
}

impl Worker {
    fn start(plugin: &Plugin) -> Result<Self> {
        let mut child = Command::new(worker_executable()?)
            .arg(CC1_WORKER_COMMAND)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("CC1 音声計測プロセスを起動できません")?;
        let input = child.stdin.take().expect("stdin was piped");
        let output = child.stdout.take().expect("stdout was piped");
        let (tx, replies) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(output);
            let mut line = Vec::new();
            while reader
                .read_until(b'\n', &mut line)
                .is_ok_and(|len| len != 0)
            {
                if let Some(payload) = line.strip_prefix(REPLY_PREFIX.as_bytes()) {
                    if tx.send(serde_json::from_slice(payload)).is_err() {
                        break;
                    }
                }
                line.clear();
            }
        });
        let mut worker = Self {
            child,
            input,
            replies,
        };
        send_json(&mut worker.input, plugin)?;
        Ok(worker)
    }

    fn measure(&mut self, request: &Request) -> Result<Reply> {
        send_json(&mut self.input, request)?;
        match self.replies.recv_timeout(PATCH_TIMEOUT) {
            Ok(reply) => Ok(reply.context("CC1 音声計測プロセスの応答が不正です")?),
            Err(RecvTimeoutError::Timeout) => {
                bail!(
                    "CC1=0/127 の音声比較が {} 秒以内に完了しませんでした",
                    PATCH_TIMEOUT.as_secs()
                )
            }
            Err(RecvTimeoutError::Disconnected) => {
                let status = self.child.try_wait()?;
                bail!("CC1 音声計測プロセスが応答を返さず終了しました ({status:?})")
            }
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn send_json(output: &mut impl Write, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}

#[cfg(not(test))]
fn worker_executable() -> Result<PathBuf> {
    Ok(std::env::current_exe()?)
}

// A library test harness is not the cmrt command-line binary.
#[cfg(test)]
fn worker_executable() -> Result<PathBuf> {
    std::env::var_os("CMRT_CC1_WORKER_EXE")
        .map(PathBuf::from)
        .context("CMRT_CC1_WORKER_EXE に cmrt バイナリを指定してください")
}

pub fn run_cc1_worker() -> Result<()> {
    let mut lines = io::stdin().lock().lines();
    let plugin: Plugin = serde_json::from_str(&lines.next().transpose()?.context("CC1 plugin")?)?;
    if plugin.id == cmrt_core::SURGE_XT_PLUGIN_ID {
        // Use the same headless data layout as the playback server, before
        // loading CLAP or starting any worker threads.
        let _ = cmrt_core::apply_minimal_surge_data_home();
    }
    let entry = load_entry(&plugin.path);
    for line in lines {
        let request = serde_json::from_str(&line?)?;
        let reply: Reply = match &entry {
            Ok(entry) => audio::observe(&plugin, entry, request).map_err(|err| format!("{err:#}")),
            Err(error) => Err(format!("{error:#}")),
        };
        let mut output = io::stdout().lock();
        write!(output, "\n{REPLY_PREFIX}")?;
        send_json(&mut output, &reply)?;
    }
    Ok(())
}
