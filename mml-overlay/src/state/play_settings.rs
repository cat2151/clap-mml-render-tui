//! 演奏設定（`Ctrl+L`）の値の出入口。開閉とキー処理は音色 selector の部品が持つ
//! （音色選択を開いている最中にも開けるため）。

use cmrt_patch_select::PlaySettings;

use super::MmlOverlay;

impl MmlOverlay<'_> {
    /// いまの演奏設定。演奏を積む経路とセッション保存がこれを見る。
    pub fn play_settings(&self) -> PlaySettings {
        self.patch_audition_select.play_settings()
    }

    /// セッションから復元した演奏設定を入れる。起動時に1度だけ呼ぶ。
    pub fn set_restored_play_settings(&mut self, settings: PlaySettings) {
        self.patch_audition_select.set_play_settings(settings);
    }

    #[cfg(test)]
    pub(crate) fn play_settings_select(&self) -> Option<&cmrt_patch_select::PlaySettingsSelect> {
        self.patch_audition_select.play_settings_select()
    }
}
