use std::sync::Mutex;

use super::*;

fn wav_bytes_i16(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let cursor = Cursor::new(&mut bytes);
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(cursor, spec).unwrap();
        for sample in samples {
            writer.write_sample(*sample).unwrap();
        }
        writer.finalize().unwrap();
    }
    bytes
}

#[test]
fn decode_wav_bytes_accepts_16bit_stereo() {
    let bytes = wav_bytes_i16(48_000, 2, &[0, i16::MAX, i16::MIN, 0]);

    let samples = decode_wav_bytes(&bytes, 48_000).unwrap();

    assert_eq!(samples.len(), 4);
    assert_eq!(samples[0], 0.0);
    assert!((samples[1] - 1.0).abs() < f32::EPSILON);
    assert!(samples[2] <= -1.0);
}

#[test]
fn decode_wav_bytes_rejects_unexpected_sample_rate() {
    let bytes = wav_bytes_i16(44_100, 2, &[0, 0]);

    let error = decode_wav_bytes(&bytes, 48_000).unwrap_err();

    assert!(error.to_string().contains("expected 48000Hz"));
}

/// 注入された sink が受け取った行。sink は `fn` ポインタなのでキャプチャできず、static で受ける。
static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn capture(line: &str) {
    CAPTURED.lock().unwrap().push(line.to_string());
}

/// sink 注入漏れは「ログが黙って消える」形で失敗するため、経路そのものを固定しておく。
#[test]
fn injected_sink_receives_prefixed_lines() {
    set_log_sink(capture);
    CAPTURED.lock().unwrap().clear();

    log_offline_render_event("event=render mml=cde");

    assert_eq!(
        CAPTURED.lock().unwrap().as_slice(),
        ["offline-render: event=render mml=cde".to_string()]
    );
}

#[test]
fn truncate_for_log_appends_ellipsis_beyond_the_limit() {
    assert_eq!(truncate_for_log("abcdef", 3), "abc...");
    assert_eq!(truncate_for_log("abc", 3), "abc");
}
