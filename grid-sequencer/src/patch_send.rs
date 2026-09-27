//! 音色を server へ準備させる 3 つの口。どれも [`crate::auto_reverb::GridAutoReverb::patch`] で
//! chain を決めてから sender へ渡す。sender の準備 API をこれ以外から呼ばないこと。

use crate::GridSequencerScreen;

/// テストで観測する、送った準備 1 件。
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SentPatch {
    /// `"prepare"`・`"preload"`、または行差し替えの理由。
    pub(crate) reason: &'static str,
    pub(crate) instance_id: u8,
    pub(crate) patch: crate::GridPatch,
}

impl GridSequencerScreen {
    /// 鳴っている bank の全 instance の音色を差し替える。
    pub(crate) fn send_prepare_all(&self) {
        let patches = self
            .state
            .patches()
            .map(|(instance_id, patch)| (instance_id, self.auto_reverb.patch(patch)))
            .collect::<Vec<_>>();
        #[cfg(test)]
        for (instance_id, patch) in &patches {
            self.record_sent_patch("prepare", *instance_id, patch);
        }
        if let Some(sender) = &self.midi_sender {
            sender.prepare(patches);
        }
    }

    /// 鳴っている bank の 1 行だけ音色を差し替える。sender が無ければ `None`。
    pub(crate) fn send_row_patch(
        &self,
        row: usize,
        instance_id: u8,
        patch: Option<&str>,
        reason: &'static str,
    ) -> Option<u64> {
        let patch = self.auto_reverb.patch(patch);
        #[cfg(test)]
        self.record_sent_patch(reason, instance_id, &patch);
        self.midi_sender
            .as_ref()
            .map(|sender| sender.set_row_patch(row, instance_id, patch, reason))
    }

    /// 待機 bank の 1 instance へ音色を先読みさせる。
    pub(crate) fn send_preload(&self, instance_id: u8, patch: Option<&str>) {
        let patch = self.auto_reverb.patch(patch);
        #[cfg(test)]
        self.record_sent_patch("preload", instance_id, &patch);
        if let Some(sender) = &self.midi_sender {
            sender.preload(instance_id, patch);
        }
    }

    #[cfg(test)]
    fn record_sent_patch(&self, reason: &'static str, instance_id: u8, patch: &crate::GridPatch) {
        self.sent_patches.borrow_mut().push(SentPatch {
            reason,
            instance_id,
            patch: patch.clone(),
        });
    }
}
