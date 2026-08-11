//! 画面が共有ランタイムから受け取る情報一式と、その読み取り。
//!
//! 画面側は config も patch 一覧も自分では持たない。app 側の glue
//! （`tui::grid_sequencer_glue`）が毎フレーム組み立てて渡す。

use cmrt_chord::ChordProgressionCatalog;

use crate::GridVoicingLookup;

/// patch 一覧のバックグラウンド読み込み状態のスナップショット。
/// 共有ランタイム側の `PatchLoadState` から glue が変換して渡す。
pub enum GridPatchLoad<'a> {
    Loading,
    Ready(&'a [(String, String)]),
    Err(&'a str),
}

/// grid sequencer 画面が共有ランタイムから受け取る情報一式。
pub struct GridSequencerContext<'a> {
    pub patch_dirs_configured: bool,
    pub patch_load: GridPatchLoad<'a>,
    /// chord mode が進行を抽選するカタログ。空なら chord mode は開始できない。
    pub chord_catalog: &'a ChordProgressionCatalog,
    /// 和音用 patch の当たり判定に使う mono/poly 判定。
    pub voicing: &'a dyn GridVoicingLookup,
    /// 和音に使う patch のカテゴリ（config.toml の `chord_patch_categories`）。
    /// 空ならカテゴリでは絞らない。
    pub chord_patch_categories: &'a [String],
    /// bass 行に使う patch のカテゴリ（config.toml の `bass_patch_categories`）。
    /// 空ならカテゴリでは絞らない。
    pub bass_patch_categories: &'a [String],
    /// アルペジオ行に使う patch のカテゴリ（config.toml の `arpeggio_patch_categories`）。
    /// 空ならカテゴリでは絞らない。
    pub arpeggio_patch_categories: &'a [String],
    /// drum 行に使う patch のカテゴリ（config.toml の `drum_patch_categories`）。4 役で共通。
    /// 空ならカテゴリでは絞らない。
    pub drum_patch_categories: &'a [String],
    /// kick 行に使う patch の名前キーワード（config.toml の `kick_patch_keywords`）。
    /// 小文字化して渡すこと。空ならキーワードでは絞らない。
    pub kick_patch_keywords: &'a [String],
    /// snare 行に使う patch の名前キーワード（config.toml の `snare_patch_keywords`）。
    pub snare_patch_keywords: &'a [String],
    /// hi-hat 行に使う patch の名前キーワード（config.toml の `hihat_patch_keywords`）。
    pub hihat_patch_keywords: &'a [String],
    /// コード進行カタログが更新されたか（再起動アナウンスの合図。一度だけ true）。
    pub chord_source_updated: bool,
}

impl GridSequencerContext<'_> {
    /// ランダム選択に使える patch 一覧。読み込み中・エラー時は空を返す。
    pub(crate) fn patches(&self) -> &[(String, String)] {
        match &self.patch_load {
            GridPatchLoad::Ready(pairs) => pairs,
            GridPatchLoad::Loading | GridPatchLoad::Err(_) => &[],
        }
    }

    pub(crate) fn patch_status(&self) -> GridPatchStatus {
        if !self.patch_dirs_configured {
            return GridPatchStatus::NotConfigured;
        }
        match &self.patch_load {
            GridPatchLoad::Ready(pairs) => GridPatchStatus::Ready(pairs.len()),
            GridPatchLoad::Loading => GridPatchStatus::Loading,
            GridPatchLoad::Err(error) => GridPatchStatus::Err((*error).to_string()),
        }
    }

    pub(crate) fn patches_are_loading(&self) -> bool {
        matches!(self.patch_load, GridPatchLoad::Loading)
    }

    pub(crate) fn patches_are_ready(&self) -> bool {
        matches!(self.patch_load, GridPatchLoad::Ready(_))
    }
}

/// ステータス行に出す patch 一覧の状態（直近のランダム化時点のスナップショット）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum GridPatchStatus {
    #[default]
    Loading,
    Ready(usize),
    NotConfigured,
    Err(String),
}

impl GridPatchStatus {
    pub fn label(&self) -> String {
        match self {
            Self::Loading => "patches loading".to_string(),
            Self::Ready(count) => format!("{count} patches"),
            Self::NotConfigured => "patches_dirs 未設定".to_string(),
            Self::Err(error) => format!("patches error: {error}"),
        }
    }
}
