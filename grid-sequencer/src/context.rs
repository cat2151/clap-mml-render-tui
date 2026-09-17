//! 画面が共有ランタイムから受け取る情報一式と、その読み取り。
//!
//! 画面側は config も patch 一覧も自分では持たない。app 側の glue
//! （`tui::grid_sequencer_glue`）が毎フレーム組み立てて渡す。

use std::{borrow::Cow, collections::BTreeMap};

use cmrt_chord::ChordProgressionCatalog;
use cmrt_patches::PatchRoleIndex;
use cmrt_tui_core::patch_load::{PatchLoadMeasurement, PatchLoadState};

use crate::GridVoicingLookup;

/// grid sequencer 画面が共有ランタイムから受け取る情報一式。
pub struct GridSequencerContext<'a> {
    pub patch_dirs_configured: bool,
    /// patch 一覧のバックグラウンド読み込み状態。共有ランタイムの物をそのまま借りる。
    pub patch_load: &'a PatchLoadState,
    /// catalog 構築時に計測した patch ごとのロード時間。auto random の ETA に使う。
    /// cache 読み込み前や cache を使わない診断では `None`。
    pub load_measurements: Option<&'a BTreeMap<String, PatchLoadMeasurement>>,
    /// chord mode が進行を抽選するカタログ。空なら chord mode は開始できない。
    pub chord_catalog: &'a ChordProgressionCatalog,
    /// 和音用 patch の当たり判定に使う mono/poly 判定。
    pub voicing: &'a dyn GridVoicingLookup,
    /// MML selectorと共有する、最新の排他的Role分類索引。
    pub patch_roles: Cow<'a, PatchRoleIndex>,
    /// コード進行カタログが更新されたか（再起動アナウンスの合図。一度だけ true）。
    pub chord_source_updated: bool,
    /// 設定不足でカタログから外れたプラグインの案内。patch selector の枠下へ出す。
    ///
    /// 一覧に**出てこない**ものの話なので、`patch_load` をいくら見ても分からない。
    /// 空なら 1 文字も表示しない。
    pub catalog_notes: &'a [String],
}

impl GridSequencerContext<'_> {
    /// ランダム選択に使える patch 一覧。読み込み中・エラー時は空を返す。
    pub(crate) fn patches(&self) -> &[(String, String)] {
        match self.patch_load {
            PatchLoadState::Ready(snapshot) => snapshot.pairs(),
            PatchLoadState::Loading | PatchLoadState::Err(_) => &[],
        }
    }

    pub(crate) fn patch_status(&self) -> GridPatchStatus {
        if !self.patch_dirs_configured {
            return GridPatchStatus::NotConfigured;
        }
        match self.patch_load {
            PatchLoadState::Ready(snapshot) => GridPatchStatus::Ready(snapshot.pairs().len()),
            PatchLoadState::Loading => GridPatchStatus::Loading,
            PatchLoadState::Err(error) => GridPatchStatus::Err(error.clone()),
        }
    }

    pub(crate) fn patches_are_loading(&self) -> bool {
        matches!(self.patch_load, PatchLoadState::Loading)
    }

    pub(crate) fn patches_are_ready(&self) -> bool {
        matches!(self.patch_load, PatchLoadState::Ready(_))
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
