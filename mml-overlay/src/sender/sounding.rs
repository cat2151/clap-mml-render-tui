//! いま音源で鳴っているものの、ただ 1 つの記録。
//!
//! オーバーレイの音は 2 系統ある（打鍵の生 MIDI と、live timeline へ積んだ行の演奏）。
//! 「次の音を鳴らすときは、それまでの演奏を止める」を守るには、止める側が両方を
//! 知っていなければならない。以前は生 MIDI を [`crate::state`] 側が、timeline を
//! [`super::line_playback::LinePlayback`] が別々に持っていて、片方が「自分は鳴らして
//! いない」と早期 return すると**サーバーへ 1 つもコマンドが飛ばない**経路ができた。
//! 打鍵の音が鳴りっぱなしになるのはそこ。記録をここ 1 つへ寄せて塞ぐ。
//!
//! 記録が実態と合っているかは note number ごとの重なり数で監視する。同じ note number
//! へ note on を 2 回出すと note off 1 回では止まらないので、深さが 2 以上になった時点で
//! 状態機械が破れている。破れを見つけたら以後の停止を音源リセット
//! （`stop_live_all`）へ格上げして、耳に聞こえる被害を出さずに log だけ残す。

use std::collections::BTreeMap;

use super::log_line;
use crate::{NOTE_OFF, NOTE_ON};

/// 生 MIDI と live timeline の両方をまとめた「鳴っているもの」の記録。
#[derive(Default)]
pub(super) struct Sounding {
    /// 生 MIDI で note on を出した note number と、その重なり数。
    /// 正常なら値は必ず 1。2 以上は状態機械の破れ。
    typed: BTreeMap<u8, u32>,
    /// live timeline へ積んだ演奏が残っているか。
    /// timeline の音は note off では止まらないので、真なら音源リセットが要る。
    timeline: bool,
    /// 記録と実態がずれた疑いがあるか。ずれたら次の停止を音源リセットへ格上げする。
    suspect: bool,
}

impl Sounding {
    /// 鳴っているものが 1 つも無いか。
    pub(super) fn is_silent(&self) -> bool {
        self.typed.is_empty() && !self.timeline
    }

    /// 止めるのに音源リセットが要るか。
    ///
    /// timeline の音は個別の note off では止まらない。記録がずれている疑いがあるときも、
    /// 記録から作った note off は当てにならないのでリセットへ倒す。
    pub(super) fn needs_hard_stop(&self) -> bool {
        self.timeline || self.suspect
    }

    /// 生 MIDI で送ったメッセージを、送った順のまま記録へ反映する。
    ///
    /// 記録は「送ったもの」に厳密に追随させる。送る側と記録する側が別の計算を
    /// すると、そのズレがそのまま鳴りっぱなしになる。
    pub(super) fn record_sent(&mut self, messages: &[[u8; 3]]) {
        for message in messages {
            match note_event(message) {
                Some((NOTE_ON, pitch)) => self.record_note_on(pitch),
                Some((_, pitch)) => self.record_note_off(pitch),
                None => {}
            }
        }
    }

    fn record_note_on(&mut self, pitch: u8) {
        let depth = {
            let depth = self.typed.entry(pitch).or_insert(0);
            *depth += 1;
            *depth
        };
        if depth > 1 {
            // ここに来たら、この note number は note off 1 回では止まらない。
            self.mark_suspect(&format!("duplicate-note-on note={pitch} depth={depth}"));
        }
    }

    fn record_note_off(&mut self, pitch: u8) {
        match self.typed.get_mut(&pitch) {
            Some(depth) if *depth > 1 => *depth -= 1,
            Some(_) => {
                self.typed.remove(&pitch);
            }
            // 鳴らしていない音への note off。害は無いが記録がずれている証拠。
            None => self.mark_suspect(&format!("note-off-without-note-on note={pitch}")),
        }
    }

    /// timeline へ演奏を積んだ。
    pub(super) fn begin_timeline(&mut self) {
        self.timeline = true;
    }

    /// 記録に残っている生 MIDI の音を全部止める note off。
    pub(super) fn note_offs(&self) -> Vec<[u8; 3]> {
        self.typed
            .iter()
            .flat_map(|(pitch, depth)| std::iter::repeat_n([NOTE_OFF, *pitch, 0], *depth as usize))
            .collect()
    }

    /// 音源が黙っている前提へ戻す。停止コマンドを送った直後にだけ呼ぶ。
    ///
    /// `hard` は音源リセット（`stop_live_all`）で止めたか。リセットなら実態が
    /// 確実に黙ったので、ずれの疑いもここで晴れる。
    pub(super) fn clear(&mut self, hard: bool) {
        self.typed.clear();
        self.timeline = false;
        if hard {
            self.suspect = false;
        }
    }

    /// 記録と実態がずれた疑いを立てる。以後の停止は音源リセットになる。
    pub(super) fn mark_suspect(&mut self, reason: &str) {
        self.suspect = true;
        log_line(format!(
            "action=mml-overlay-sounding event=suspect {reason}"
        ));
    }

    /// log 用の 1 行。どの note number が何重に鳴っている扱いかまで出す。
    pub(super) fn describe(&self) -> String {
        let typed = self
            .typed
            .iter()
            .map(|(pitch, depth)| format!("{pitch}x{depth}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "typed=[{typed}] timeline={} suspect={}",
            self.timeline, self.suspect
        )
    }
}

/// note on / note off なら `(status, note number)`。それ以外は `None`。
///
/// velocity 0 の note on は note off と同じ意味なので、そちらへ寄せる。
fn note_event(message: &[u8; 3]) -> Option<(u8, u8)> {
    match message[0] {
        NOTE_ON if message[2] > 0 => Some((NOTE_ON, message[1])),
        NOTE_ON | NOTE_OFF => Some((NOTE_OFF, message[1])),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
