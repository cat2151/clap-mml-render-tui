use std::time::Instant;

use super::KeyboardScreen;

impl KeyboardScreen<'_> {
    /// 周期送信・patch切替後の再送・音出し確認ガイドを1フレーム分進める。
    ///
    /// 日次 overlay を新たに表示した場合だけ true を返す（保存は共有ランタイム側の責務）。
    pub fn pump_periodic(&mut self, now: Instant, local_date: &str) -> bool {
        let ready = self.connection_status().phase.accepts_notes();
        let first_overlay_today = self.note_guide.tick(now, ready, local_date);
        if !ready {
            return first_overlay_today;
        }
        // patch切替後の現在値再送(refresh) → 周期送信、の順で1回のsendにまとめる
        let mut messages = self.state.take_pending_refresh_messages(now);
        messages.extend(self.state.poll_periodic(now));
        if !messages.is_empty() {
            if let Some(sender) = &self.midi_sender {
                sender.send(messages, self.state.patch());
            }
        }
        first_overlay_today
    }
}
