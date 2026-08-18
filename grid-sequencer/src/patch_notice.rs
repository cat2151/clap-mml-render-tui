//! PATCH 欄を押しても selector が開けなかったときに出す、理由の通知。
//!
//! grid sequencer には notepad のような通知欄が無く、手掛かりはステータス行の
//! `p:0` / `p:none` しかない。押しても何も起きないと利用者が原因を切り分けられない
//! ので、理由を中央 overlay で数秒だけ出す。
//!
//! 演奏中の画面なので入力は塞がない（`help` や再起動アナウンスと違い、読み終える
//! まで操作を止める理由が無い）。時間で自然に消し、次に開けたら即座に消す。

use std::time::{Duration, Instant};

use crate::{GridPatchLoad, GridSequencerContext};

/// 読み切れるだけの長さ。再起動アナウンス（3 秒）と違い操作を塞がないので、
/// 目を離していても拾えるよう少し長く出す。
pub(crate) const PATCH_NOTICE_DURATION: Duration = Duration::from_secs(5);

/// patch selector を開けなかった理由。[`crate::patch_selector::PatchSelector::new`]
/// が組み立てる。理由の判定をここ以外に持たない（同じ条件を 2 か所で書くと必ずずれる）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PatchUnavailable {
    /// config.toml に `patches_dirs` が無い。音色置き場を持たないプラグイン
    /// （Dexed など）では常にこれになる。
    NotConfigured,
    /// 一覧をバックグラウンドで読み込み中。待てば開ける。
    Loading,
    /// 一覧の読み込みに失敗した。
    LoadError(String),
    /// 読み込めたが 1 件も無い。
    NoPatches,
    /// 和音行なので poly の音色だけに絞ったら 0 件になった。
    NoPolyPatches,
    /// 一覧はあるが、この行の用途（chord / bass / drum など）に合う音色が 0 件だった。
    NoRolePatches,
}

/// 一覧そのものが使えない理由。1 件以上あって使える状態なら `None`。
///
/// 判定はここ 1 か所だけに置く。selector を開く経路と PATCH 欄の wheel 送りの
/// 両方から呼ぶので、条件を書き分けると必ずずれる。
pub(crate) fn catalog_unavailable(ctx: &GridSequencerContext<'_>) -> Option<PatchUnavailable> {
    if !ctx.patch_dirs_configured {
        return Some(PatchUnavailable::NotConfigured);
    }
    match &ctx.patch_load {
        GridPatchLoad::Loading => Some(PatchUnavailable::Loading),
        GridPatchLoad::Err(error) => Some(PatchUnavailable::LoadError((*error).to_string())),
        GridPatchLoad::Ready([]) => Some(PatchUnavailable::NoPatches),
        GridPatchLoad::Ready(_) => None,
    }
}

impl PatchUnavailable {
    /// overlay に出す行。1 行目だけで何が起きたか分かる順に並べる。
    pub(crate) fn lines(&self) -> Vec<String> {
        match self {
            Self::NotConfigured => vec![
                "音色を選べません".to_string(),
                "config.toml の patches_dirs が未設定です".to_string(),
            ],
            Self::Loading => vec![
                "音色一覧を読み込み中です".to_string(),
                "読み込みが終わるともう一度開けます".to_string(),
            ],
            Self::LoadError(error) => vec![
                "音色一覧の読み込みに失敗しました".to_string(),
                error.clone(),
            ],
            Self::NoPatches => vec![
                "音色を選べません".to_string(),
                "音色一覧が 0 件です".to_string(),
                "config.toml の patches_dirs を確認してください".to_string(),
            ],
            Self::NoPolyPatches => vec![
                "和音に使える音色がありません".to_string(),
                "この行は poly の音色だけを使います".to_string(),
            ],
            Self::NoRolePatches => vec![
                "この行に使える音色がありません".to_string(),
                "用途に合う音色が 1 件も見つかりません".to_string(),
                "config.toml のカテゴリ設定を確認してください".to_string(),
            ],
        }
    }
}

/// 表示中の通知と、出し始めた時刻。
pub(crate) struct PatchNotice {
    pub(crate) reason: PatchUnavailable,
    since: Instant,
}

impl PatchNotice {
    pub(crate) fn new(reason: PatchUnavailable, now: Instant) -> Self {
        Self { reason, since: now }
    }

    pub(crate) fn expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.since) >= PATCH_NOTICE_DURATION
    }
}

#[cfg(test)]
mod tests;
