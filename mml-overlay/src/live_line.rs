//! 先頭 JSON 込みの MML 1 行を、LIVE で鳴らす「音色 + effect chain + 演奏」へ変える。
//!
//! 画面の試聴と CLI の計測（`cmrt live-line-check`）が同じ行を同じ形で送るよう、
//! 変換はここ 1 か所に置く。

use crate::line_play::{line_events, LineProgram, LineStatus};
use crate::patch_json::{effect_chain_json, patch_name, strip_patch_json};
use crate::LivePatch;

/// [`crate::MmlOverlaySender::play_line`] へそのまま渡せる 1 行。
#[derive(Clone, Debug)]
pub struct LiveLine {
    pub patch: LivePatch,
    pub program: LineProgram,
}

/// `source` の行頭 JSON から音色と `"effects after instrument"` を、残りの MML から
/// 1 回だけ鳴らす演奏を作る。空行・演奏にできない MML・読めない行頭 JSON は `Err`。
pub fn live_line(source: &str) -> Result<LiveLine, String> {
    let (status, performance) = line_events(strip_patch_json(source));
    match status {
        LineStatus::Played { .. } => {}
        LineStatus::Idle => return Err(format!("行 {source:?} が空です")),
        LineStatus::Error(error) => {
            return Err(format!("行 {source:?} を演奏データにできません: {error}"))
        }
    }
    let patch = patch_name(source);
    Ok(LiveLine {
        patch: LivePatch::with_effect_chain(patch.as_deref(), &effect_chain_json(source)?),
        program: LineProgram::once(performance),
    })
}

#[cfg(test)]
mod tests;
