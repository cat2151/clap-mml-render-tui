use super::*;

#[test]
fn scan_loops_progress_and_summary_are_printed() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let path = std::path::PathBuf::from("loops").join("Kick.wav");

    write_scan_progress(
        &loop_library::LoopScanProgress::Started { roots: 2 },
        &mut stdout,
        &mut stderr,
    )
    .unwrap();
    write_scan_progress(
        &loop_library::LoopScanProgress::Analyzing {
            current: 3,
            total: 7,
            path: path.clone(),
        },
        &mut stdout,
        &mut stderr,
    )
    .unwrap();
    write_scan_progress(
        &loop_library::LoopScanProgress::Skipped {
            path,
            error: "RIFF/WAVE形式ではありません".to_string(),
        },
        &mut stdout,
        &mut stderr,
    )
    .unwrap();
    write_scan_summary(
        loop_library::LoopScanSummary {
            roots: 2,
            wav_files: 6,
            skipped_wav_files: 1,
        },
        &mut stdout,
    )
    .unwrap();

    let stdout = String::from_utf8(stdout).unwrap();
    let stderr = String::from_utf8(stderr).unwrap();
    assert!(stdout.contains("WAVループ走査を開始します: 2 roots"));
    assert!(stdout.contains("[3/7] WAVを解析:"));
    assert!(stdout.contains("2 roots / 6 indexed WAV / 1 skipped WAV"));
    assert!(stderr.contains("警告: WAVをスキップしました:"));
    assert!(stderr.contains("RIFF/WAVE形式ではありません"));
}
