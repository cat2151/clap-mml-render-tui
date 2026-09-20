//! EFFECT CHAIN overlay（`x`）でユーザーへ直接表示する文言。

pub(crate) const OVERLAY_TITLE: &str = " EFFECT CHAIN ";
pub(crate) const ADD_OVERLAY_TITLE: &str = " EFFECT CHAIN: add preset ";

pub(crate) const INSTRUMENT_LABEL: &str = "instrument: ";
pub(crate) const EMPTY_CHAIN: &str = "(effect なし。a で追加)";
pub(crate) const NO_PRESETS: &str = "effect plugin が見つからない";
pub(crate) const NOT_AVAILABLE_ON_THIS_BACKEND: &str = "この backend では effect を使えない";
pub(crate) const PLAYABLE_TRACK_ONLY: &str = "effect chain は演奏 track でのみ使用できます";

pub(crate) const FOOTER: &str = "j/k:移動  a:追加  dd:削除  Enter:確定(再render)  ESC:破棄";
pub(crate) const ADD_FOOTER: &str = "j/k:移動  Enter:末尾へ追加  ESC:戻る";
pub(crate) const STATUS: &str = "EFFECT CHAIN  j/k:移動  a:追加  dd:削除  Enter:確定  ESC:破棄";
pub(crate) const ADD_STATUS: &str = "EFFECT CHAIN ADD  j/k:移動  Enter:末尾へ追加  ESC:戻る";
