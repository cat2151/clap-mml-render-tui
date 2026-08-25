//! `Ctrl+L` の演奏設定。
//!
//! 設定は MML overlay 全体で共通なので、音色選択を開いている最中にも開けなければ
//! ならない。そのため開閉のキー判定は [`MmlOverlay::handle_key`] の**先頭**、
//! つまり音色選択やフレーズ履歴への委譲より手前に置く。
//!
//! ここは値を持つだけで、鳴っている演奏へは触らない。積み直すのは次に演奏を
//! 積む経路（[`MmlOverlay::play_current_line`]）の仕事。

use crossterm::event::KeyEvent;

use crate::play_settings::{
    is_play_settings_trigger, PlaySettings, PlaySettingsAction, PlaySettingsSelect,
};

use super::{MmlOverlay, MmlOverlayAction};

impl MmlOverlay<'_> {
    /// いまの演奏設定。演奏を積む経路とセッション保存がこれを見る。
    pub fn play_settings(&self) -> PlaySettings {
        self.play_settings
    }

    /// セッションから復元した演奏設定を入れる。起動時に1度だけ呼ぶ。
    pub fn set_restored_play_settings(&mut self, settings: PlaySettings) {
        self.play_settings = settings;
    }

    pub(crate) fn play_settings_select(&self) -> Option<&PlaySettingsSelect> {
        self.play_settings_select.as_ref()
    }

    /// 演奏設定がキーを食べたなら、その結果を返す。
    ///
    /// モーダルが開いている間はすべてのキーを吸う（最も手前のモーダル）。閉じている
    /// ときは開くキーだけを拾い、それ以外は `None` を返して後段の判定へ流す。
    pub(super) fn intercept_play_settings_key(
        &mut self,
        key: KeyEvent,
    ) -> Option<MmlOverlayAction> {
        if self.play_settings_select.is_some() {
            return Some(self.handle_play_settings_key(key));
        }
        if is_play_settings_trigger(key) {
            self.open_play_settings();
            return Some(MmlOverlayAction::Continue);
        }
        None
    }

    fn open_play_settings(&mut self) {
        self.play_settings_select = Some(PlaySettingsSelect::open(self.play_settings));
        crate::log_line(format!(
            "action=play-settings event=open {}",
            describe(&self.play_settings)
        ));
    }

    fn handle_play_settings_key(&mut self, key: KeyEvent) -> MmlOverlayAction {
        let Some(select) = self.play_settings_select.as_mut() else {
            return MmlOverlayAction::Continue;
        };
        match select.handle_key(key) {
            PlaySettingsAction::Continue => MmlOverlayAction::Continue,
            PlaySettingsAction::Confirm(settings) => self.close_play_settings(settings, "confirm"),
            PlaySettingsAction::Cancel(settings) => self.close_play_settings(settings, "cancel"),
        }
    }

    /// 取り消しでも「開いた時点の値」が載って戻るので、採用は 1 か所で足りる。
    fn close_play_settings(&mut self, settings: PlaySettings, result: &str) -> MmlOverlayAction {
        self.play_settings_select = None;
        self.play_settings = settings;
        crate::log_line(format!(
            "action=play-settings event=close result={result} {}",
            describe(&settings)
        ));
        MmlOverlayAction::Continue
    }
}

fn describe(settings: &PlaySettings) -> String {
    format!(
        "repeat={} modulation={} velocity={}",
        settings.repeat, settings.filters.modulation, settings.filters.velocity
    )
}
