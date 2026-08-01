//! 出力バッファを上限まで厚くしても解消しない、慢性的なフレームドロップの検出。
//!
//! [`super::adaptive_buffer::AdaptiveBuffer`] が「バッファをどう厚くするか」を持つのに対し、
//! ここは「厚くしても駄目だと諦める境目」だけを持つ。
//!
//! chord mode のダブルバッファリング（[`crate::cycle_swap`]）は、演奏の裏で待機 bank へ
//! patch を仕込めるだけの余裕がサーバー側にあることを前提にしている。16 track のように
//! その前提が崩れる環境では、上限倍率に張り付いたままフレームドロップが続く。そこを
//! 検出して、画面側をシングルバッファリング（[`crate::single_buffer`]）へ落とす。

use std::time::{Duration, Instant};

/// フレームドロップの連続が途切れたとみなす間隔。
/// 「1秒に1回以上」を、観測の間隔がこれを超えないこと、として表す。
const DROP_GAP: Duration = Duration::from_secs(1);
/// 連続がこの長さに達したら成立。
const SUSTAINED_WINDOW: Duration = Duration::from_secs(10);

/// 上限倍率でのフレームドロップが続いているかを見張る。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct OverloadDetector {
    /// いま続いている連続の起点。`None` なら連続していない。
    streak_since: Option<Instant>,
    /// 直近でフレームドロップを観測した時刻。
    last_drop: Option<Instant>,
    /// 一度立ったら降ろさない。降ろすと、シングルへ落ちて静かになった直後に
    /// ダブルへ戻り、また荒れる、を往復してしまう。
    tripped: bool,
}

impl OverloadDetector {
    pub(super) fn new() -> Self {
        Self {
            streak_since: None,
            last_drop: None,
            tripped: false,
        }
    }

    /// 出力バッファの観測ごと（50ms 周期）に呼ぶ。成立した瞬間だけ true を返す。
    ///
    /// `at_max` は **倍率を追従させる前の**梯子が上限だったかどうか。梯子の途中で出た
    /// ドロップはまだ厚くする余地があるので数えない（上げた直後に上限になっても、
    /// そのドロップは1段下での出来事）。
    pub(super) fn observe(&mut self, now: Instant, at_max: bool, new_underrun_frames: u64) -> bool {
        if self.tripped {
            return false;
        }
        if !at_max {
            self.reset_streak();
            return false;
        }
        if new_underrun_frames > 0 {
            let continues = self
                .last_drop
                .is_some_and(|last| now.saturating_duration_since(last) <= DROP_GAP);
            if !continues {
                self.streak_since = Some(now);
            }
            self.last_drop = Some(now);
        } else if self
            .last_drop
            .is_some_and(|last| now.saturating_duration_since(last) > DROP_GAP)
        {
            // 1秒ドロップが無かった。連続は切れたので数え直す。
            self.reset_streak();
            return false;
        }
        let Some(since) = self.streak_since else {
            return false;
        };
        if now.saturating_duration_since(since) < SUSTAINED_WINDOW {
            return false;
        }
        self.tripped = true;
        true
    }

    fn reset_streak(&mut self) {
        self.streak_since = None;
        self.last_drop = None;
    }
}

#[cfg(test)]
mod tests;
