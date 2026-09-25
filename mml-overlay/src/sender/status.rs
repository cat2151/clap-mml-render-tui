use std::time::Instant;

/// sender worker の現在状態。TUI は読み取りだけ行う。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MmlOverlaySenderStatus {
    pub(crate) command_id: u64,
    pub(crate) loading: bool,
    pub(crate) loading_patch: Option<String>,
    pub(crate) sounding: Vec<u8>,
    /// worker が準備と timeline への送信を終えた、直近の行演奏。
    pub(crate) line_playback: Option<MmlOverlayLinePlayback>,
    /// 直近の音源準備が失敗した理由。成功したら消える。
    ///
    /// **無音の理由を持っているのは worker だけ**。準備が終わるまで画面に出ている
    /// 「音が鳴るまで」の overlay は `loading` が下りた瞬間に消えるので、
    /// 消えた理由をここから持ち帰れないと「黙って消えて音も出ない」になる。
    pub(crate) prepare_error: Option<String>,
    /// `prepare_error` を出した command。別の command の失敗を取り違えないために持つ。
    pub(crate) prepare_error_command_id: u64,
}

impl MmlOverlaySenderStatus {
    pub fn command_id(&self) -> u64 {
        self.command_id
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn loading_patch(&self) -> Option<&str> {
        self.loading_patch.as_deref()
    }

    pub fn sounding(&self) -> &[u8] {
        &self.sounding
    }

    /// 準備と timeline への送信まで成功した、現在の行演奏。
    ///
    /// command を受け取っただけでは `Some` にならない。無音行、送信失敗、停止、または
    /// 別 command に置き換わった場合は `None` になる。
    pub fn line_playback(&self) -> Option<MmlOverlayLinePlayback> {
        self.line_playback
    }

    /// 直近の音源準備が失敗した理由。成功していれば `None`。
    pub fn prepare_error(&self) -> Option<&str> {
        self.prepare_error.as_deref()
    }

    /// command `command_id` の音源準備が失敗した理由。その command が失敗していなければ `None`。
    pub fn prepare_error_for(&self, command_id: u64) -> Option<&str> {
        (self.prepare_error_command_id == command_id)
            .then_some(self.prepare_error.as_deref())
            .flatten()
    }
}

/// sender worker が実際に timeline へ積んだ行演奏の実時間区間。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MmlOverlayLinePlayback {
    pub(crate) command_id: u64,
    pub(crate) started_at: Instant,
    /// `None` は repeat により明示的な終了時刻が無い演奏。
    pub(crate) ends_at: Option<Instant>,
}

impl MmlOverlayLinePlayback {
    pub fn command_id(self) -> u64 {
        self.command_id
    }

    pub fn started_at(self) -> Instant {
        self.started_at
    }

    pub fn ends_at(self) -> Option<Instant> {
        self.ends_at
    }

    pub fn is_sounding_at(self, now: Instant) -> bool {
        now >= self.started_at && self.ends_at.is_none_or(|ends_at| now < ends_at)
    }
}
