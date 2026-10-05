//! 音色 selector と演奏設定を 1 つにまとめ、試聴を action にする部品。
//!
//! 何を鳴らすかは持つ側が決める（[`PatchSelectOutcome::Audition`] で問い合わせる）。

mod play_settings;

use std::collections::BTreeMap;

use cmrt_patches::{PatchRole, PatchRoleIndex};
use cmrt_tui_core::patch_load::PatchLoadMeasurement;
use crossterm::event::KeyEvent;
use serde_json::Value;

use crate::auto_reverb::AutoReverbRules;
use crate::patch_audition::{audition_action, PatchAudition, PatchAuditionAction, PatchChange};
use crate::patch_select::{AutoReverbHost, PatchSelect, PatchSelectAction, PatchSelectRequest};
use crate::play_settings::{PlaySettings, PlaySettingsSelect};
use crate::PatchCatalogSnapshot;

/// selector を開けなかった理由。持つ側の画面へ出す。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchCatalogNotice {
    Loading,
    Empty,
    Error(String),
}

impl PatchCatalogNotice {
    /// 画面へ出す文言。
    pub fn message(&self) -> String {
        match self {
            Self::Loading => "音色一覧を読み込み中です。完了後に自動で開きます".to_string(),
            Self::Empty => "選択できる音色がありません".to_string(),
            Self::Error(error) => format!("音色一覧の読み込みに失敗: {error}"),
        }
    }
}

/// 試聴を持つ側へ問い合わせる時点。持つ側はこれで鳴らすものを選び分ける。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditionMoment {
    /// 一覧のカーソルが動いた。
    Candidate,
    /// `Space` で選択中の音色を試聴する。
    Replay,
}

/// selector がキーを処理した結果、持つ側へ求める処理。
pub enum PatchSelectOutcome {
    /// 表示が変わっただけ。
    Continue,
    /// `moment` に応じて鳴らすものを決め、[`PatchAuditionSelect::audition_action`] へ渡す。
    Audition {
        moment: AuditionMoment,
        patch: PatchChange,
    },
    /// ユーザー追加プリセットを JSON へ保存させる。`preview` は絞り込みが変わって移った
    /// 先頭候補で、保存に続けてその音色で試聴する（鳴らすものは持つ側が決める）。
    SavePresets {
        presets: Vec<(String, String)>,
        preview: Option<String>,
    },
    /// auto reverb の設定を保存させる。`preview` はカーソルの音色で、掛ける reverb が
    /// 変わったので同じ音色でも鳴らし直す。
    SaveAutoReverb {
        rules: AutoReverbRules,
        preview: Option<String>,
    },
    /// selector が閉じた。`confirmed` は `Enter` で確定したか。`restore` は取り消しで
    /// 開いたときの音色へ戻す action（試聴で音色を動かしていなければ `None`）。
    Closed {
        confirmed: bool,
        restore: Option<PatchAuditionAction>,
    },
}

/// selector を開くときに持つ側から渡すスナップショット。
#[derive(Default)]
pub struct PatchAuditionContext {
    pub catalog: PatchCatalogSnapshot,
    /// Grid Sequencer と共有する、同じ catalog 世代の Role 索引。
    pub patch_role_index: PatchRoleIndex,
    /// 今の音色の Role が分からないときに開く Role。`None` なら `ALL`。
    /// catalog の Loading 完了待ちを挟んでも、この指定を使って開く。
    pub initial_role: Option<PatchRole>,
    /// catalog 構築時に計測した patch 別の load 結果。
    pub load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    /// `(Grid Sequencer 上の役割 group, 正規表現)` のユーザー追加プリセット。
    pub filter_presets: Vec<(String, String)>,
    /// 設定不足でカタログから外れたプラグインの案内。selector の枠の下へ出す。
    pub catalog_notes: Vec<String>,
    /// 音色 favorite。登録が新しい順。`★ Favorite` の Preset と ★ 列に出す。
    pub favorites: Vec<String>,
}

/// 音色 selector ＋ 演奏設定 ＋ 試聴の action 化。
///
/// 音色と演奏設定は開き直しでは消えない（持つ側がセッションへ保存する）。
#[derive(Default)]
pub struct PatchAuditionSelect<'a> {
    /// いまの音色。selector の確定と、持つ側の差し替えだけが書き換える。
    patch: Option<String>,
    /// 最後に確定したときの絞り込み。次に開く selector の Regex 欄の初期値で、`patch` と同じく
    /// 確定と持つ側の差し替えだけが書き換える。
    query: String,
    catalog: PatchCatalogSnapshot,
    role_index: PatchRoleIndex,
    initial_role: Option<PatchRole>,
    /// Load 列へ渡す、開いている catalog と同世代の計測結果。
    load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    filter_presets: Vec<(String, String)>,
    catalog_notes: Vec<String>,
    favorites: Vec<String>,
    /// selector を開けなかった理由。標準 stream ではなく持つ側の画面へ出す。
    notice: Option<PatchCatalogNotice>,
    /// selector を開くときに渡す auto reverb。`None` の host では扱わない。
    auto_reverb: Option<AutoReverbHost>,
    /// 最後に確定した音色に掛かっていた auto reverb の 1 段。
    confirmed_auto_reverb: Option<Value>,
    /// Loading 中の open 要求を、一覧完成後に自動で実行する予約。
    requested: bool,
    select: Option<PatchSelect<'a>>,
    play_settings: PlaySettings,
    play_settings_select: Option<PlaySettingsSelect>,
}

impl<'a> PatchAuditionSelect<'a> {
    /// 開いている間だけ持つスナップショットを入れ替え、selector とモーダルを閉じる。
    pub fn open(&mut self, context: PatchAuditionContext) {
        self.catalog = context.catalog;
        self.role_index = context.patch_role_index;
        self.initial_role = context.initial_role;
        self.load_measurements = context.load_measurements;
        self.filter_presets = context.filter_presets;
        self.catalog_notes = context.catalog_notes;
        self.favorites = context.favorites;
        self.notice = None;
        self.auto_reverb = None;
        self.confirmed_auto_reverb = None;
        self.requested = false;
        self.select = None;
        self.play_settings_select = None;
    }

    /// 次に開く selector で auto reverb を扱わせる。[`Self::open`] の後に呼ぶ。
    pub fn set_auto_reverb(&mut self, host: Option<AutoReverbHost>) {
        self.auto_reverb = host;
    }

    /// 開いている selector で `display` を試聴するときに chain へ足す 1 段。
    pub fn auto_reverb_stage(&self, display: &str) -> Option<Value> {
        self.select.as_ref()?.auto_reverb_stage(display)
    }

    /// 最後に `Enter` で確定した音色に掛かっていた auto reverb の 1 段。
    pub fn confirmed_auto_reverb_stage(&self) -> Option<&Value> {
        self.confirmed_auto_reverb.as_ref()
    }

    /// 開いている間だけ持っていたスナップショットを手放す。音色と演奏設定は残す。
    pub fn release(&mut self) {
        self.catalog = PatchCatalogSnapshot::Loading;
        self.role_index = PatchRoleIndex::default();
        self.filter_presets = Vec::new();
        self.favorites = Vec::new();
        self.select = None;
        self.notice = None;
        self.requested = false;
        self.play_settings_select = None;
    }

    /// いまの音色。セッション保存はこれを見る。
    pub fn patch(&self) -> Option<&str> {
        self.patch.as_deref()
    }

    /// 音色を差し替える（セッションからの復元、履歴の取り込み）。
    pub fn set_patch(&mut self, patch: Option<String>) {
        self.patch = patch;
    }

    /// 最後に確定したときの絞り込み（plugin solo/mute を含む）。
    pub fn query(&self) -> &str {
        &self.query
    }

    /// 次に開く selector の絞り込みを差し替える（セッションからの復元）。
    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }

    pub fn select(&self) -> Option<&PatchSelect<'a>> {
        self.select.as_ref()
    }

    pub fn is_select_open(&self) -> bool {
        self.select.is_some()
    }

    pub fn notice(&self) -> Option<&PatchCatalogNotice> {
        self.notice.as_ref()
    }

    /// 音色 selector を開く。catalog が Loading なら完了後の open を予約する。
    pub fn request_select(&mut self) {
        self.notice = None;
        self.requested = false;
        match &self.catalog {
            PatchCatalogSnapshot::Loading => {
                self.notice = Some(PatchCatalogNotice::Loading);
                self.requested = true;
                crate::log_line(
                    "action=patch-select event=open result=waiting reason=catalog-loading"
                        .to_string(),
                );
            }
            PatchCatalogSnapshot::Error(error) => {
                self.notice = Some(PatchCatalogNotice::Error(error.clone()));
                crate::log_line(format!(
                    "action=patch-select event=open result=blocked reason=catalog-error detail={error:?}"
                ));
            }
            PatchCatalogSnapshot::Ready(patches) if patches.is_empty() => {
                self.notice = Some(PatchCatalogNotice::Empty);
                crate::log_line(
                    "action=patch-select event=open result=blocked reason=catalog-empty"
                        .to_string(),
                );
            }
            PatchCatalogSnapshot::Ready(patches) => {
                let count = patches.len();
                self.select = PatchSelect::open(PatchSelectRequest {
                    patches: patches.clone(),
                    current: self.patch.clone(),
                    user_presets: self.filter_presets.clone(),
                    role_index: self.role_index.clone(),
                    initial_role: self.initial_role,
                    drum_kit_only: false,
                    catalog_notes: self.catalog_notes.clone(),
                    load_measurements: self.load_measurements.clone(),
                    favorites: self.favorites.clone(),
                    initial_query: self.query.clone(),
                    auto_reverb: self.auto_reverb.clone(),
                });
                let selected = self.select.as_ref().and_then(PatchSelect::selected);
                crate::log_line(format!(
                    "action=patch-select event=open result=success count={count} query={:?} selected={selected:?}",
                    self.query
                ));
            }
        }
    }

    /// Loading だった一覧を差し替える。Loading 中に open が要求されていれば、ここで開く。
    pub fn sync_catalog(
        &mut self,
        catalog: PatchCatalogSnapshot,
        patch_role_index: PatchRoleIndex,
        load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    ) {
        if !self.is_waiting_for_catalog() {
            return;
        }
        let requested = self.requested;
        let result = match &catalog {
            PatchCatalogSnapshot::Loading => return,
            PatchCatalogSnapshot::Ready(patches) => format!("ready count={}", patches.len()),
            PatchCatalogSnapshot::Error(error) => format!("error detail={error:?}"),
        };
        self.catalog = catalog;
        self.role_index = patch_role_index;
        self.load_measurements = load_measurements;
        crate::log_line(format!(
            "action=patch-catalog event=sync result={result} open_requested={requested}"
        ));
        if requested {
            self.request_select();
        }
    }

    pub fn is_waiting_for_catalog(&self) -> bool {
        matches!(&self.catalog, PatchCatalogSnapshot::Loading)
    }

    /// 開いている selector へキーを渡す。
    pub fn handle_select_key(&mut self, key: KeyEvent) -> PatchSelectOutcome {
        let Some(select) = self.select.as_mut() else {
            return PatchSelectOutcome::Continue;
        };
        let audition = |moment, patch| PatchSelectOutcome::Audition {
            moment,
            patch: PatchChange::Switch(Some(patch)),
        };
        match select.handle_key(key) {
            PatchSelectAction::Continue => PatchSelectOutcome::Continue,
            PatchSelectAction::Preview(patch) => audition(AuditionMoment::Candidate, patch),
            PatchSelectAction::PlayLine(patch) => audition(AuditionMoment::Replay, patch),
            PatchSelectAction::Confirm(patch) => {
                self.confirmed_auto_reverb = select.auto_reverb_stage(&patch);
                self.query = select.committed_query().to_string();
                // 試聴で読み込み済みの音色がそのまま残るので、ここでは積み直さない。
                self.select = None;
                self.patch = Some(patch);
                PatchSelectOutcome::Closed {
                    confirmed: true,
                    restore: None,
                }
            }
            PatchSelectAction::SaveUserPresets { presets, preview } => {
                self.filter_presets.clone_from(&presets);
                PatchSelectOutcome::SavePresets { presets, preview }
            }
            PatchSelectAction::SaveAutoReverb { rules, preview } => {
                if let Some(host) = self.auto_reverb.as_mut() {
                    host.rules.clone_from(&rules);
                }
                PatchSelectOutcome::SaveAutoReverb { rules, preview }
            }
            PatchSelectAction::Cancel => self.cancel_select(),
        }
    }

    fn cancel_select(&mut self) -> PatchSelectOutcome {
        let Some(select) = self.select.take() else {
            return PatchSelectOutcome::Continue;
        };
        let restore =
            (select.previewed() != select.original()).then(|| PatchAuditionAction::SetPatch {
                patch: select.original().map(str::to_string),
                notes: None,
            });
        PatchSelectOutcome::Closed {
            confirmed: false,
            restore,
        }
    }

    /// `audition` を `patch` で、いまの演奏設定を載せて鳴らす action にする。
    pub fn audition_action(
        &self,
        audition: Option<PatchAudition>,
        patch: PatchChange,
    ) -> PatchAuditionAction {
        audition_action(audition, patch, self.patch.as_deref(), self.play_settings)
    }
}

#[cfg(test)]
mod tests;
