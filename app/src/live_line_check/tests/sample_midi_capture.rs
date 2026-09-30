//! METAL-GTX 付属のサンプル MID を、Guitar Articulation 画面の MID モードと同じ列
//! （全体は [`SampleMidi::events`]、1 音は [`SampleMidi::note_events`]）で行の LIVE 演奏経路へ送り、
//! live mix の出力が無音でないかを確かめる。CC・pitch bend の効きは比べない。
//!
//! - a: `CC22_Mute_Control.mid` の全体の頭（録る長さは [`super::capture_support`] の 1 行ぶん）
//! - b: `CC20_21_Vibrato_Control.mid` の最初の 1 音
//!
//! 付属 MID の置き場（`sfz/UI_METAL-GTX/Sample_MIDI_Files/Control_Change` の実ディレクトリ）を
//! 環境変数で渡す。無ければ skip。ほかの環境変数は [`super::capture_support`] と同じ。
//!
//! ```text
//! $env:CMRT_TEST_SAMPLE_MIDI_DIR = "<sfz 置き場>\UI_METAL-GTX\Sample_MIDI_Files\Control_Change"
//! cargo test -p clap-mml-render-tui --lib sample_midi_capture -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use cmrt_guitar_articulation::SampleMidi;

use super::capture_support::{peak, CaptureSetup};

const SAMPLE_MIDI_DIR_ENV: &str = "CMRT_TEST_SAMPLE_MIDI_DIR";
const WHOLE_FILE: &str = "CC22_Mute_Control.mid";
const NOTE_FILE: &str = "CC20_21_Vibrato_Control.mid";
/// これを超える peak があれば鳴っている（-40 dBFS）。
const SILENCE_PEAK: f64 = 0.01;

#[test]
#[ignore = "実機の sampler 音色と付属 MID が要る（CMRT_TEST_KEYSWITCH_PATCH・CMRT_TEST_SAMPLE_MIDI_DIR）"]
fn sample_midi_whole_and_single_note_are_not_silent() {
    let Some(dir) = std::env::var_os(SAMPLE_MIDI_DIR_ENV).map(PathBuf::from) else {
        eprintln!("sample-midi-capture: skip ({SAMPLE_MIDI_DIR_ENV} が無い)");
        return;
    };
    let Some(setup) = CaptureSetup::from_env("sample-midi-capture") else {
        return;
    };
    let whole = sample_midi(&dir, WHOLE_FILE);
    let note_source = sample_midi(&dir, NOTE_FILE);
    assert!(note_source.group_count() > 0, "{NOTE_FILE} に音が無い");
    let note = note_source.note_events(0);

    let a = setup.record("a-whole", &[whole.events().to_vec()]);
    let b = setup.record("b-first-note", &[note]);
    drop(setup);

    let a_peak = peak(&a[0].samples);
    let b_peak = peak(&b[0].samples);
    eprintln!(
        "sample-midi-capture: a {WHOLE_FILE} whole peak={a_peak:.4} seconds={:.3}",
        a[0].samples.len() as f64 / f64::from(a[0].sample_rate)
    );
    eprintln!(
        "sample-midi-capture: b {NOTE_FILE} note 0 peak={b_peak:.4} seconds={:.3}",
        b[0].samples.len() as f64 / f64::from(b[0].sample_rate)
    );

    assert!(a_peak > SILENCE_PEAK, "{WHOLE_FILE} の全体が鳴っていない");
    assert!(
        b_peak > SILENCE_PEAK,
        "{NOTE_FILE} の最初の 1 音が鳴っていない"
    );
}

fn sample_midi(dir: &std::path::Path, name: &str) -> SampleMidi {
    let path = dir.join(name);
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("{} を読めない: {e}", path.display()));
    let events = cmrt_chord::timed_smf_events(&bytes)
        .unwrap_or_else(|e| panic!("{} を解釈できない: {e}", path.display()))
        .events;
    SampleMidi::new(name.to_string(), events)
}
