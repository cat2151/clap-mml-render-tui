//! 行の演奏の fadeout。EFFECT CHAIN の試聴で次の候補へ移るときに使う。

use super::{log_error, log_line, MmlOverlaySender, SenderCommandKind};

impl MmlOverlaySender {
    /// 行の演奏で鳴っている音（前の行の release・effect の余韻を含む）を `fade_ms` で
    /// fadeout し、列で準備を待っている行は鳴らさせない。
    ///
    /// fadeout を送ったら `Ok(true)`、鳴らした行が無ければ `Ok(false)`。送れなかったら `Err`
    /// （音は残っているので、止めるなら [`Self::stop`]）。
    ///
    /// **fadeout は command の列に並べず、呼び出したスレッドからその場で送る。** 列の前に
    /// 音色の準備（chain 付きで数百 ms）が並んでいても待たない。続けて [`Self::play_line`]
    /// した行は fadeout した instance でも等倍で鳴る（server が新しい行の開始で絞りを解く）。
    /// [`Self::stop`] と違い server の全NoteOff を送らない（送ると fadeout が段差で切れる）。
    pub fn fade_out_line(&self, fade_ms: u32) -> Result<bool, String> {
        self.enqueue(SenderCommandKind::Supersede);
        let instance_ids = self.sounding_lines.take();
        if instance_ids.is_empty() {
            return Ok(false);
        }
        match self.fader.fade_out_instances(&instance_ids, fade_ms) {
            Ok(()) => {
                log_line(format!(
                    "action=mml-overlay-fade-out event=sent instances={instance_ids:?} fade_ms={fade_ms}"
                ));
                Ok(true)
            }
            Err(error) => {
                log_error(format!(
                    "action=mml-overlay-fade-out event=error instances={instance_ids:?} fade_ms={fade_ms} error=\"{error}\""
                ));
                Err(error)
            }
        }
    }
}
