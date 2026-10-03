//! 入力欄を持たない host が、音色 selector だけを開く持ち方。
//!
//! 試聴で鳴らすものは開くときに host が 1 つ決めて渡し、開いた直後・候補移動・`Space`・
//! preset 保存時の先頭候補のどれでもそれを鳴らす。host の試聴の仕様（DAW の meas、
//! Chord Chart の section）はカーソル位置と無関係なので、時点で選び分けない。
//!
//! 開いているかどうかは host が `Option<DirectPatchSelect>` で持つ。`Some` の間は
//! 一覧の Loading 待ちも含めて開いている。

use std::collections::BTreeMap;

use cmrt_patches::PatchRoleIndex;
use cmrt_tui_core::patch_load::PatchLoadMeasurement;
use crossterm::event::{KeyCode, KeyEvent};
use serde_json::Value;

use crate::auto_reverb::AutoReverbRules;
use crate::patch_audition::{PatchAudition, PatchAuditionAction, PatchChange};
use crate::play_settings::PlaySettings;
use crate::{
    AutoReverbHost, PatchAuditionContext, PatchAuditionSelect, PatchCatalogNotice,
    PatchCatalogSnapshot, PatchSelectOutcome,
};

/// 開くときに host が決めて渡すもの。
pub struct DirectPatchSelectRequest {
    pub context: PatchAuditionContext,
    /// 今の音色。開いた直後の試聴と、selector の初期カーソルに使う。
    pub patch: Option<String>,
    /// 絞り込みの初期値。前回閉じたときの [`DirectPatchSelect::query`] を渡せば絞り込みが戻る。
    pub query: String,
    /// host の画面全体で共通の演奏設定。閉じたら [`DirectPatchSelect::play_settings`] を書き戻す。
    pub play_settings: PlaySettings,
    /// 試聴で鳴らすもの。`None` は「鳴らすものが無いので音色の差し替えだけ」。
    pub audition: Option<PatchAudition>,
    /// auto reverb を扱う host だけが渡す。`None` なら表示行も `e`/`E` も出ない。
    pub auto_reverb: Option<AutoReverbHost>,
}

/// キーを処理した結果、host へ求める処理。
#[derive(Debug, PartialEq)]
pub enum DirectSelectOutcome {
    Continue,
    /// この試聴を鳴らす。
    Play(PatchAuditionAction),
    /// ユーザー追加プリセットを保存し、続けて `preview` を鳴らす。
    SavePresets {
        presets: Vec<(String, String)>,
        preview: Option<PatchAuditionAction>,
    },
    /// auto reverb の設定を保存し、続けて `preview` を鳴らす（同じ音色でも鳴らし直す）。
    SaveAutoReverb {
        rules: AutoReverbRules,
        preview: Option<PatchAuditionAction>,
    },
    /// 閉じた。host は `Option` を `None` に戻し、`restore` があれば鳴らす（音色を戻す）。
    Closed {
        confirmed: bool,
        restore: Option<PatchAuditionAction>,
    },
}

/// 入力欄なしで開く音色 selector。
pub struct DirectPatchSelect<'a> {
    select: PatchAuditionSelect<'a>,
    audition: Option<PatchAudition>,
}

impl<'a> DirectPatchSelect<'a> {
    /// selector を開き、開いた直後に今の音色で鳴らす action を返す。
    ///
    /// 一覧が Loading なら完了後に開く予約をして、試聴は今すぐ返す（待つ間も今の音色を
    /// 聴ける）。Error / 空で開けなければ試聴は `None` で、理由は [`Self::notice`] にある
    /// （その理由を見せたまま待つか、すぐ閉じるかは host が決める）。
    pub fn open(request: DirectPatchSelectRequest) -> (Self, Option<PatchAuditionAction>) {
        let mut select = PatchAuditionSelect::default();
        select.open(request.context);
        select.set_patch(request.patch.clone());
        select.set_query(request.query);
        select.set_play_settings(request.play_settings);
        select.set_auto_reverb(request.auto_reverb);
        select.request_select();
        let opening = (select.is_select_open() || select.is_waiting_for_catalog()).then(|| {
            select.audition_action(request.audition.clone(), PatchChange::Switch(request.patch))
        });
        let direct = Self {
            select,
            audition: request.audition,
        };
        (direct, opening)
    }

    /// 演奏設定を最初に見る（音色選択の最中にも開けるモーダルのため）。selector が開いて
    /// いない間（一覧の Loading 待ち・開けなかった理由の表示中）は、閉じる `Esc` 以外の打鍵を受けない。
    pub fn handle_key(&mut self, key: KeyEvent) -> DirectSelectOutcome {
        if self.select.intercept_play_settings_key(key) {
            return DirectSelectOutcome::Continue;
        }
        if !self.select.is_select_open() {
            if key.code == KeyCode::Esc {
                return DirectSelectOutcome::Closed {
                    confirmed: false,
                    restore: None,
                };
            }
            return DirectSelectOutcome::Continue;
        }
        match self.select.handle_select_key(key) {
            PatchSelectOutcome::Continue => DirectSelectOutcome::Continue,
            PatchSelectOutcome::Audition { patch, .. } => {
                DirectSelectOutcome::Play(self.audition_action(patch))
            }
            PatchSelectOutcome::SavePresets { presets, preview } => {
                DirectSelectOutcome::SavePresets {
                    presets,
                    preview: preview
                        .map(|patch| self.audition_action(PatchChange::Switch(Some(patch)))),
                }
            }
            PatchSelectOutcome::SaveAutoReverb { rules, preview } => {
                DirectSelectOutcome::SaveAutoReverb {
                    rules,
                    preview: preview
                        .map(|patch| self.audition_action(PatchChange::Switch(Some(patch)))),
                }
            }
            PatchSelectOutcome::Closed { confirmed, restore } => {
                DirectSelectOutcome::Closed { confirmed, restore }
            }
        }
    }

    fn audition_action(&self, patch: PatchChange) -> PatchAuditionAction {
        self.select.audition_action(self.audition.clone(), patch)
    }

    /// 今の音色。候補を動かしただけでは変わらず、`Enter` の確定でだけ変わる。
    pub fn patch(&self) -> Option<&str> {
        self.select.patch()
    }

    /// `Enter` で確定したときの絞り込み。確定するまでは開くときに渡した値のまま。
    pub fn query(&self) -> &str {
        self.select.query()
    }

    /// 開いている selector で `display` を試聴するときに chain へ足す auto reverb の 1 段。
    pub fn auto_reverb_stage(&self, display: &str) -> Option<Value> {
        self.select.auto_reverb_stage(display)
    }

    /// `Enter` で確定した音色に掛かっていた auto reverb の 1 段。
    pub fn confirmed_auto_reverb_stage(&self) -> Option<&Value> {
        self.select.confirmed_auto_reverb_stage()
    }

    pub fn play_settings(&self) -> PlaySettings {
        self.select.play_settings()
    }

    /// 開くときに host が渡した試聴。
    pub fn audition(&self) -> Option<&PatchAudition> {
        self.audition.as_ref()
    }

    /// selector を開けていない理由。
    pub fn notice(&self) -> Option<&PatchCatalogNotice> {
        self.select.notice()
    }

    pub fn is_select_open(&self) -> bool {
        self.select.is_select_open()
    }

    pub fn is_waiting_for_catalog(&self) -> bool {
        self.select.is_waiting_for_catalog()
    }

    /// Loading だった一覧を差し替え、予約していれば selector を開く。
    pub fn sync_catalog(
        &mut self,
        catalog: PatchCatalogSnapshot,
        patch_role_index: PatchRoleIndex,
        load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    ) {
        self.select
            .sync_catalog(catalog, patch_role_index, load_measurements);
    }

    pub(crate) fn audition_select(&self) -> &PatchAuditionSelect<'a> {
        &self.select
    }
}

#[cfg(test)]
mod tests;
