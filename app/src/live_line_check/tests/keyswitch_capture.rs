//! sampler の key switch が、行の LIVE 演奏経路（`play_line` → realtime play server）で
//! 効くかを、live mix の出力（`CMRT_LIVE_CAPTURE_WAV`）で確かめる。
//!
//! 同じ 3 音を「KS なし」「各音の前に KS」「2 音目の前だけ別の KS」で鳴らし、音ごとの
//! 波形の相関を比べる。音色の round robin の位置を揃えるため、条件ごとに server を起こし直す。
//! 続けて同じ server で 2 行を鳴らし、1 行目で押した KS が 2 行目まで残るかも測る。
//! 2 行目どうしは、それまでの履歴が同じ 2 つの server の間でだけ比べる（別の履歴と
//! 比べると round robin の位置の差が混ざる）。
//!
//! 実機の音色が要るので通常は skip。音色（`patches_dirs` からの相対）を環境変数で渡す:
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! $env:CMRT_TEST_KEYSWITCH_PATCH = "sfz/<library>/Programs/<program>.sfz"
//! $env:CMRT_TEST_KEYSWITCH_OUT_DIR = "<録った WAV を残す dir>"   # 省略時は temp
//! cargo test -p clap-mml-render-tui --lib keyswitch_capture -- --ignored --nocapture
//! ```
//!
//! KS の note number は METAL-GTX の配置（17 = Sus_Down、26 = Hammer-On）。

use super::capture_support::{peak, rms, CaptureSetup, Segment, DIFFERENT, SAME};

const SUS_DOWN: u8 = 17;
const HAMMER_ON: u8 = 26;
const PITCHES: [u8; 3] = [40, 42, 43];
const NOTE_SECONDS: f64 = 0.5;

/// 各音の直前に押す KS。`None` はその音で KS を押さない。
type KeySwitches = [Option<u8>; 3];

const PLAIN: KeySwitches = [None, None, None];
const SUS_EACH: KeySwitches = [Some(SUS_DOWN), Some(SUS_DOWN), Some(SUS_DOWN)];
const HAMMER_SECOND: KeySwitches = [None, Some(HAMMER_ON), None];

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn key_switches_change_the_sound_on_the_live_line_path() {
    let Some(setup) = CaptureSetup::from_env("keyswitch-capture") else {
        return;
    };
    let record = |label: &str, lines: &[KeySwitches]| {
        setup.record(label, &lines.iter().map(phrase).collect::<Vec<_>>())
    };
    let plain = record("a-plain", &[PLAIN]);
    let sus_each = record("b-sus-each", &[SUS_EACH]);
    let hammer = record("c-hammer-second", &[HAMMER_SECOND]);
    let hammer_then_plain = record("d-hammer-then-plain", &[HAMMER_SECOND, PLAIN]);
    let hammer_then_sus = record("e-hammer-then-sus", &[HAMMER_SECOND, SUS_EACH]);
    drop(setup);

    let a_b = compare("a vs b", &plain[0], &sus_each[0]);
    let a_c = compare("a vs c", &plain[0], &hammer[0]);
    let c_d = compare("c vs d(1 行目)", &hammer[0], &hammer_then_plain[0]);
    let d_e = compare(
        "d(2 行目) vs e(2 行目)",
        &hammer_then_plain[1],
        &hammer_then_sus[1],
    );

    assert!(
        a_b.iter().all(|ncc| *ncc > SAME),
        "KS 17 を足すと音が変わった"
    );
    assert!(a_c[0] > SAME, "c の 1 音目が a と違う");
    assert!(a_c[1] < DIFFERENT, "KS 26 を押した 2 音目が a と同じ音");
    assert!(
        c_d.iter().all(|ncc| *ncc > SAME),
        "同じ入力を起こし直した server で鳴らすと音が変わる（比べ方が壊れている）"
    );
    assert!(
        d_e[0] < DIFFERENT,
        "KS なしの 2 行目が KS 17 の 2 行目と同じ音（前の行の KS が残っていない）"
    );
}

/// 3 音の単音フレーズ。同時刻は note off → KS → 演奏音の順に積む。
fn phrase(switches: &KeySwitches) -> Vec<cmrt_chord::TimedMidiEvent> {
    let event = |seconds: f64, message: [u8; 3]| cmrt_chord::TimedMidiEvent { seconds, message };
    let mut events = Vec::new();
    let mut held: Vec<u8> = Vec::new();
    for (index, (pitch, switch)) in PITCHES.iter().zip(switches).enumerate() {
        let at = index as f64 * NOTE_SECONDS;
        for key in held.drain(..) {
            events.push(event(at, [0x80, key, 0]));
        }
        if let Some(switch) = switch {
            events.push(event(at, [0x90, *switch, 127]));
            held.push(*switch);
        }
        events.push(event(at, [0x90, *pitch, 100]));
        held.push(*pitch);
    }
    let end = PITCHES.len() as f64 * NOTE_SECONDS;
    for key in held {
        events.push(event(end, [0x80, key, 0]));
    }
    events
}

/// 音ごとに、ずれを探した最大の正規化相関を出して返す。RMS と頭 30ms の peak も並べる。
fn compare(label: &str, left: &Segment, right: &Segment) -> Vec<f64> {
    (0..PITCHES.len())
        .map(|index| {
            let at = index as f64 * NOTE_SECONDS;
            let ncc = left.correlation(right, at, NOTE_SECONDS);
            let (l, r) = (left.window(at, NOTE_SECONDS), right.window(at, NOTE_SECONDS));
            eprintln!(
                "keyswitch-capture: {label} note={} ncc={ncc:.4} rms={:.4}/{:.4} head_peak={:.4}/{:.4}",
                index + 1,
                rms(l),
                rms(r),
                peak(left.window(at, 0.030)),
                peak(right.window(at, 0.030)),
            );
            ncc
        })
        .collect()
}
