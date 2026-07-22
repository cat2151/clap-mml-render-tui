use clap_mml_render_tui::loop_browser::library::{LoopScanProgress, LoopScanSummary};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Default)]
struct Snapshot {
    current: usize,
    total: usize,
    path: Option<PathBuf>,
    stage: String,
    indexed: usize,
    skipped: usize,
    last_file_skipped: bool,
}

enum Command {
    Finish(String),
}

pub(super) struct ScanProgressLog {
    sender: Option<mpsc::Sender<Command>>,
    handle: Option<thread::JoinHandle<std::io::Result<()>>>,
    snapshot: Arc<Mutex<Snapshot>>,
}

impl ScanProgressLog {
    pub(super) fn start(path: &Path, interval: Duration) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        let mut writer = BufWriter::new(file);
        write_line(&mut writer, "scan-loops: started")?;
        let (sender, receiver) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot {
            stage: "starting".to_string(),
            ..Snapshot::default()
        }));
        let logger_snapshot = Arc::clone(&snapshot);
        let handle = thread::Builder::new()
            .name("scan-loops-progress-log".to_string())
            .spawn(move || run_logger(receiver, writer, logger_snapshot, interval))?;
        Ok(Self {
            sender: Some(sender),
            handle: Some(handle),
            snapshot,
        })
    }

    pub(super) fn observe(&mut self, event: &LoopScanProgress) {
        let mut snapshot = self.snapshot.lock().unwrap();
        match event {
            LoopScanProgress::Started { .. } => snapshot.stage = "discovering".to_string(),
            LoopScanProgress::Analyzing {
                current,
                total,
                path,
            } => {
                if *current > snapshot.current
                    && snapshot.current > 0
                    && !snapshot.last_file_skipped
                {
                    snapshot.indexed += 1;
                }
                snapshot.current = *current;
                snapshot.total = *total;
                snapshot.path = Some(path.clone());
                snapshot.stage = "metadata".to_string();
                snapshot.last_file_skipped = false;
            }
            LoopScanProgress::Visualizing { bin, bins } => {
                snapshot.stage = format!("visualization {bin}/{bins}");
            }
            LoopScanProgress::Skipped { path, .. } => {
                snapshot.path = Some(path.clone());
                snapshot.stage = "skipped".to_string();
                snapshot.skipped += 1;
                snapshot.last_file_skipped = true;
            }
        }
    }

    pub(super) fn finish(mut self, summary: LoopScanSummary) -> std::io::Result<()> {
        self.shutdown(format!(
            "scan-loops: completed roots={} indexed={} skipped={}",
            summary.roots, summary.wav_files, summary.skipped_wav_files
        ))
    }

    pub(super) fn fail(mut self, error: &dyn std::fmt::Display) -> std::io::Result<()> {
        self.shutdown(format!("scan-loops: failed: {error}"))
    }

    fn shutdown(&mut self, line: String) -> std::io::Result<()> {
        let sender = self.sender.take().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "進捗log threadが終了しました",
            )
        })?;
        sender.send(Command::Finish(line)).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "進捗logへ完了通知できません",
            )
        })?;
        drop(sender);
        join_logger(self.handle.take())
    }
}

impl Drop for ScanProgressLog {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(Command::Finish("scan-loops: aborted".to_string()));
        }
        let _ = join_logger(self.handle.take());
    }
}

fn run_logger(
    receiver: mpsc::Receiver<Command>,
    mut writer: BufWriter<File>,
    snapshot: Arc<Mutex<Snapshot>>,
    interval: Duration,
) -> std::io::Result<()> {
    let started = Instant::now();
    let mut next_heartbeat = started + interval;
    loop {
        let timeout = next_heartbeat.saturating_duration_since(Instant::now());
        match receiver.recv_timeout(timeout) {
            Ok(Command::Finish(line)) => {
                write_line(&mut writer, &line)?;
                return Ok(());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                write_heartbeat(&mut writer, &snapshot.lock().unwrap(), started.elapsed())?;
                next_heartbeat += interval;
                while next_heartbeat <= Instant::now() {
                    next_heartbeat += interval;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                write_line(&mut writer, "scan-loops: aborted")?;
                return Ok(());
            }
        }
        if Instant::now() >= next_heartbeat {
            write_heartbeat(&mut writer, &snapshot.lock().unwrap(), started.elapsed())?;
            next_heartbeat += interval;
        }
    }
}

fn write_heartbeat(
    writer: &mut BufWriter<File>,
    snapshot: &Snapshot,
    elapsed: Duration,
) -> std::io::Result<()> {
    let path = snapshot
        .path
        .as_deref()
        .map_or_else(|| "-".to_string(), |path| path.display().to_string());
    write_line(
        writer,
        &format!(
            "scan-loops: [{}/{}] stage={} elapsed={}s indexed={} skipped={} path={path}",
            snapshot.current,
            snapshot.total,
            snapshot.stage,
            elapsed.as_secs(),
            snapshot.indexed,
            snapshot.skipped
        ),
    )
}

fn write_line(writer: &mut BufWriter<File>, message: &str) -> std::io::Result<()> {
    writeln!(writer, "[{}] {message}", timestamp_jst())?;
    writer.flush()
}

fn timestamp_jst() -> String {
    use chrono::{FixedOffset, Utc};
    Utc::now()
        .with_timezone(&FixedOffset::east_opt(9 * 60 * 60).expect("valid JST offset"))
        .format("%Y-%m-%d %H:%M:%S JST")
        .to_string()
}

fn join_logger(handle: Option<thread::JoinHandle<std::io::Result<()>>>) -> std::io::Result<()> {
    let Some(handle) = handle else {
        return Ok(());
    };
    handle
        .join()
        .map_err(|_| std::io::Error::other("進捗log threadがpanicしました"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log() -> PathBuf {
        std::env::temp_dir().join(format!(
            "cmrt_scan_progress_{}_{}.log",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn heartbeat_is_flushed_before_logger_finishes() {
        let path = temp_log();
        let mut logger = ScanProgressLog::start(&path, Duration::from_millis(10)).unwrap();
        logger.observe(&LoopScanProgress::Analyzing {
            current: 2,
            total: 7,
            path: PathBuf::from("loops/Kick.wav"),
        });
        std::thread::sleep(Duration::from_millis(30));
        let during = std::fs::read_to_string(&path).unwrap();
        assert!(during.contains("[2/7]"));
        assert!(during.contains("stage=metadata"));
        assert!(during.contains("loops/Kick.wav"));

        logger
            .finish(LoopScanSummary {
                roots: 1,
                wav_files: 7,
                skipped_wav_files: 0,
            })
            .unwrap();
        let complete = std::fs::read_to_string(&path).unwrap();
        assert!(complete.contains("completed roots=1 indexed=7 skipped=0"));
        std::fs::remove_file(path).unwrap();
    }
}
