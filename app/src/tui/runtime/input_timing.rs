//! キーを受けてから、その結果を描き終えるまでの所要時間をログへ出す。
//!
//! 1 frame は「pump → 描画 → キー待ち → キー処理」の順に回る。キー処理の時間は次の frame の
//! 先頭で、pump と描画の時間はその frame の描画直後で確定する。

use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;

use crate::logging::global_log_sink;
use crate::screen_switch::PrimaryScreen;

/// キーを伴わない frame でも、pump と描画がこれ以上かかったらログへ出す。
const SLOW_FRAME: Duration = Duration::from_millis(100);

struct PendingKey {
    key: String,
    screen: PrimaryScreen,
    received: Instant,
    handled: Option<Duration>,
}

#[derive(Default)]
pub(super) struct InputTiming {
    pending: Option<PendingKey>,
    frame_started: Option<Instant>,
    draw_started: Option<Instant>,
}

impl InputTiming {
    /// キーを読んだ直後に呼ぶ。
    pub(super) fn key_received(&mut self, key: &KeyEvent, screen: PrimaryScreen) {
        self.pending = Some(PendingKey {
            key: format!("{:?}{}", key.code, modifiers_suffix(key)),
            screen,
            received: Instant::now(),
            handled: None,
        });
    }

    /// frame の先頭（pump の前）で呼ぶ。
    pub(super) fn frame_started(&mut self) {
        let now = Instant::now();
        if let Some(pending) = self.pending.as_mut() {
            pending.handled.get_or_insert(now - pending.received);
        }
        self.frame_started = Some(now);
    }

    /// 描画の直前で呼ぶ。
    pub(super) fn draw_started(&mut self) {
        self.draw_started = Some(Instant::now());
    }

    /// 描画の直後で呼ぶ。
    pub(super) fn draw_finished(&mut self) {
        let now = Instant::now();
        let (Some(frame_started), Some(draw_started)) =
            (self.frame_started.take(), self.draw_started.take())
        else {
            return;
        };
        let pump = draw_started - frame_started;
        let draw = now - draw_started;
        match self.pending.take() {
            Some(pending) => global_log_sink(&format!(
                "input-timing: key={} screen={:?} handle_ms={} pump_ms={} draw_ms={} total_ms={}",
                pending.key,
                pending.screen,
                pending.handled.unwrap_or_default().as_millis(),
                pump.as_millis(),
                draw.as_millis(),
                (now - pending.received).as_millis(),
            )),
            None if pump + draw >= SLOW_FRAME => global_log_sink(&format!(
                "input-timing: slow-frame pump_ms={} draw_ms={}",
                pump.as_millis(),
                draw.as_millis(),
            )),
            None => {}
        }
    }
}

fn modifiers_suffix(key: &KeyEvent) -> String {
    if key.modifiers.is_empty() {
        String::new()
    } else {
        format!("+{:?}", key.modifiers)
    }
}
