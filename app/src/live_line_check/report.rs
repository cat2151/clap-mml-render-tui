//! 解析結果を表で出す。

use super::alignment::envelope_lag_ms;
use super::analysis::{detect_clicks, detect_dropouts, line_heads, Click, DEFAULT_CLICK_K};

/// 切り替え点の前側の幅。送信時刻は wall clock から読み替えるので、少し前から見る。
const SWITCH_BEFORE_MS: f64 = 50.0;
/// 切り替え点の後側の幅。先頭の音の先送り（50 ms）と出力リングの先読みを含める。
const SWITCH_AFTER_MS: f64 = 200.0;
/// 切り替え点の外のクリック候補は、この件数だけ例として出す。
const OTHER_CLICKS_SHOWN: usize = 10;
/// 頭の包絡を比べる、頭より後の長さ。次の行の張り直しで音が切られる位置（行の長さと送信の
/// 間隔で変わる）を含めないよう、頭の立ち上がりだけを見る。
const HEAD_WINDOW_MS: f64 = 60.0;
/// 頭の包絡を比べる、頭より前の長さ。無音からの立ち上がりの縁を比べる手がかりにする
/// （立ち上がりの速い音は、頭より後だけだと平らでずれが決まらない）。
const HEAD_PRE_ROLL_MS: f64 = 20.0;
/// 頭の包絡のずれを探す幅。
const HEAD_MAX_LAG_MS: f64 = 50.0;

fn ms(frames: usize, sample_rate: u32) -> f64 {
    frames as f64 * 1000.0 / f64::from(sample_rate)
}

/// 2 行目以降の送信点のうち、`frame` を切り替え点の窓に含むもの（1 始まりの行番号）。
fn switch_line(frame: usize, sends: &[usize], sample_rate: u32) -> Option<usize> {
    let at = ms(frame, sample_rate);
    sends.iter().enumerate().skip(1).find_map(|(index, &send)| {
        let send = ms(send, sample_rate);
        (at >= send - SWITCH_BEFORE_MS && at < send + SWITCH_AFTER_MS).then_some(index + 1)
    })
}

/// `label` の波形を解析して表で出す。`sends` は各行を送った frame。
pub(super) fn print(
    label: &str,
    samples: &[f32],
    sample_rate: u32,
    sends: &[usize],
    prepare_ms: Option<&[f64]>,
) {
    let frames = samples.len() / 2;
    println!();
    println!(
        "[{label}] frames={frames} duration_ms={:.1} sample_rate={sample_rate}",
        ms(frames, sample_rate)
    );
    let heads = line_heads(samples, sample_rate, sends);
    for (index, (send, head)) in sends.iter().zip(&heads).enumerate() {
        println!(
            "  line {}/{} send_ms={:.1} head_ms={} prepare_ms={}",
            index + 1,
            sends.len(),
            ms(*send, sample_rate),
            head.map_or_else(
                || "-".to_string(),
                |head| format!("{:.1}", ms(head, sample_rate))
            ),
            prepare_ms
                .and_then(|prepare| prepare.get(index))
                .map_or_else(|| "-".to_string(), |prepare| format!("{prepare:.1}")),
        );
    }

    let clicks = detect_clicks(samples, sample_rate, DEFAULT_CLICK_K);
    let (switch_clicks, other_clicks): (Vec<&Click>, Vec<&Click>) = clicks
        .iter()
        .partition(|click| switch_line(click.frame, sends, sample_rate).is_some());
    println!(
        "  clicks        : total={} at_switch={} (k={DEFAULT_CLICK_K}, 切り替え点 = 2 行目以降の送信の -{SWITCH_BEFORE_MS} ms 〜 +{SWITCH_AFTER_MS} ms)",
        clicks.len(),
        switch_clicks.len()
    );
    let shown_others = other_clicks.iter().take(OTHER_CLICKS_SHOWN);
    for click in switch_clicks.iter().chain(shown_others) {
        println!(
            "    click at_ms={:.2} magnitude={:.4} ratio={:.1} switch_line={}",
            ms(click.frame, sample_rate),
            click.magnitude,
            click.ratio,
            switch_line(click.frame, sends, sample_rate)
                .map_or_else(|| "-".to_string(), |line| line.to_string()),
        );
    }

    let dropouts = detect_dropouts(samples, sample_rate);
    let switch_dropouts = dropouts
        .iter()
        .filter(|dropout| switch_line(dropout.frame, sends, sample_rate).is_some())
        .count();
    println!(
        "  dropouts      : total={} at_switch={switch_dropouts}",
        dropouts.len()
    );
    for dropout in &dropouts {
        println!(
            "    dropout at_ms={:.2} len_ms={:.2} switch_line={}",
            ms(dropout.frame, sample_rate),
            ms(dropout.frames, sample_rate),
            switch_line(dropout.frame, sends, sample_rate)
                .map_or_else(|| "-".to_string(), |line| line.to_string()),
        );
    }
}

/// 1 波形ぶんの行の送信点。
pub(super) struct Track<'a> {
    pub(super) samples: &'a [f32],
    pub(super) sends: &'a [usize],
}

/// 各行の頭を live と offline で比べる。live の頭（`live.sends` からの時間）と、両方の頭から
/// 取った包絡のずれ（正 = live が遅れている。頭が欠けると負へ出る）を出す。
pub(super) fn print_head_alignment(
    label: &str,
    live: Track<'_>,
    offline: Track<'_>,
    sample_rate: u32,
    step_ms: u64,
    block_frames: usize,
) {
    let window_ms = HEAD_PRE_ROLL_MS + HEAD_WINDOW_MS.min(step_ms as f64);
    let pre_roll = (HEAD_PRE_ROLL_MS * f64::from(sample_rate) / 1000.0).round() as usize;
    let block_ms = ms(block_frames, sample_rate);
    println!();
    println!(
        "[{label}] timeline 開始からの live の頭と、offline との包絡のずれ（window_ms={window_ms:.0} block_ms={block_ms:.2}）"
    );
    let live_heads = line_heads(live.samples, sample_rate, live.sends);
    let offline_heads = line_heads(offline.samples, sample_rate, offline.sends);
    for (index, ((live_head, offline_head), (live_send, offline_send))) in live_heads
        .iter()
        .zip(&offline_heads)
        .zip(live.sends.iter().zip(offline.sends))
        .enumerate()
    {
        let lag = live_head
            .zip(*offline_head)
            .and_then(|(live_head, offline_head)| {
                envelope_lag_ms(
                    live.samples,
                    (live_send + live_head).saturating_sub(pre_roll),
                    offline.samples,
                    (offline_send + offline_head).saturating_sub(pre_roll),
                    sample_rate,
                    window_ms,
                    HEAD_MAX_LAG_MS,
                )
            });
        println!(
            "  line {}/{} live_head_ms={} lag_ms={} within_block={}",
            index + 1,
            live.sends.len(),
            live_head.map_or_else(
                || "-".to_string(),
                |head| format!("{:.1}", ms(head, sample_rate))
            ),
            lag.map_or_else(|| "-".to_string(), |lag| format!("{lag:.1}")),
            lag.map_or_else(
                || "-".to_string(),
                |lag| (lag.abs() <= block_ms).to_string()
            ),
        );
    }
}
