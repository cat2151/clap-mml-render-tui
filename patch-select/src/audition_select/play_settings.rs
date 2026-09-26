//! selector を開いていないときは `Ctrl+L`、開いているときは `S` で開く演奏設定。
//!
//! 設定は selector を持つ側全体で共通なので、音色選択を開いている最中にも開けなければ
//! ならない。持つ側はキーを [`PatchAuditionSelect::intercept_play_settings_key`] へ
//! **最初に**渡し、音色選択やほかのモーダルへの委譲はその後に置く。
//!
//! ここは値を持つだけで、鳴っている演奏へは触らない。積み直すのは次に演奏を積む経路の仕事。

use crossterm::event::KeyEvent;

use crate::patch_select::is_patch_select_play_settings_trigger;
use crate::play_settings::{
    is_play_settings_trigger, PlaySettings, PlaySettingsAction, PlaySettingsSelect,
};

use super::PatchAuditionSelect;

impl PatchAuditionSelect<'_> {
    /// いまの演奏設定。演奏を積む経路とセッション保存がこれを見る。
    pub fn play_settings(&self) -> PlaySettings {
        self.play_settings
    }

    /// セッションから復元した演奏設定を入れる。
    pub fn set_play_settings(&mut self, settings: PlaySettings) {
        self.play_settings = settings;
    }

    pub fn play_settings_select(&self) -> Option<&PlaySettingsSelect> {
        self.play_settings_select.as_ref()
    }

    /// 演奏設定がキーを食べたら `true`。
    ///
    /// モーダルが開いている間はすべてのキーを吸う（最も手前のモーダル）。閉じている
    /// ときは開くキーだけを拾い、それ以外は `false` を返して後段の判定へ流す。
    pub fn intercept_play_settings_key(&mut self, key: KeyEvent) -> bool {
        let trigger = match self.select.as_ref() {
            Some(select) => !select.filter_editing() && is_patch_select_play_settings_trigger(key),
            None => is_play_settings_trigger(key),
        };
        if let Some(select) = self.play_settings_select.as_mut() {
            if trigger {
                let original = select.original();
                self.close_play_settings(original, "cancel");
                return true;
            }
            match select.handle_key(key) {
                PlaySettingsAction::Continue => {}
                PlaySettingsAction::Confirm(settings) => {
                    self.close_play_settings(settings, "confirm")
                }
                PlaySettingsAction::Cancel(settings) => {
                    self.close_play_settings(settings, "cancel")
                }
            }
            return true;
        }
        if trigger {
            self.open_play_settings();
            return true;
        }
        false
    }

    fn open_play_settings(&mut self) {
        self.play_settings_select = Some(PlaySettingsSelect::open(self.play_settings));
        crate::log_line(format!(
            "action=play-settings event=open {}",
            describe(&self.play_settings)
        ));
    }

    /// 取り消しでも「開いた時点の値」が載って戻るので、採用は 1 か所で足りる。
    fn close_play_settings(&mut self, settings: PlaySettings, result: &str) {
        self.play_settings_select = None;
        self.play_settings = settings;
        crate::log_line(format!(
            "action=play-settings event=close result={result} {}",
            describe(&settings)
        ));
    }
}

fn describe(settings: &PlaySettings) -> String {
    format!(
        "repeat={} modulation={} velocity={}",
        settings.repeat, settings.filters.modulation, settings.filters.velocity
    )
}
