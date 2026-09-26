//! EFFECT CHAIN overlay（`x`）でユーザーへ直接表示する文言。

pub(crate) const OVERLAY_TITLE: &str = " EFFECT CHAIN ";
pub(crate) const ADD_OVERLAY_TITLE: &str = " EFFECT CHAIN: add preset ";
pub(crate) const REPLACE_OVERLAY_TITLE: &str = " EFFECT CHAIN: replace preset ";

pub(crate) const INSTRUMENT_LABEL: &str = "instrument: ";
pub(crate) const EMPTY_CHAIN: &str = "(effect なし。a で追加)";
pub(crate) const NO_PRESETS: &str = "effect plugin が見つからない";
pub(crate) const NOT_AVAILABLE_ON_THIS_BACKEND: &str = "effect を持たない経路では使えない";
pub(crate) const PLAYABLE_TRACK_ONLY: &str = "effect chain は演奏 track でのみ使用できます";

pub(crate) const ADD_QUERY_TITLE: &str = " query ";
pub(crate) const ADD_QUERY_TITLE_EDITING: &str = " query (Enter=確定 / ESC=中断) ";
pub(crate) const ADD_QUERY_PLACEHOLDER: &str = "/ で list を絞り込み";

pub(crate) const FOOTER: &str =
    "j/k:移動  PgUp/PgDn/Home/End:大移動  a:追加  r:差替  dd:削除  b:bypass  Alt+↑↓:並替(いずれも自動preview)  Space:preview  Enter:確定(再render)  ESC:破棄  ?:help";
pub(crate) const ADD_FOOTER: &str =
    "h/l:pane  j/k:移動(自動preview)  /:絞り込み  PgUp/PgDn/Home/End:大移動  Space:preview  b:候補だけbypassでpreview  Enter:末尾へ追加  ESC:戻る  ?:help";
pub(crate) const REPLACE_FOOTER: &str =
    "h/l:pane  j/k:移動(自動preview)  /:絞り込み  PgUp/PgDn/Home/End:大移動  Space:preview  b:候補だけbypassでpreview  Enter:差し替え  ESC:戻る  ?:help";
pub(crate) const STATUS: &str =
    "EFFECT CHAIN  j/k:移動  a:追加  r:差替  dd:削除  b:bypass  Alt+↑↓:並替(自動preview)  Space:preview  Enter:確定  ESC:破棄  ?:help";
pub(crate) const ADD_STATUS: &str =
    "EFFECT CHAIN ADD  h/l:pane  j/k:移動(自動preview)  /:絞り込み  Space:preview  b:bypass preview  Enter:末尾へ追加  ESC:戻る  ?:help";
pub(crate) const REPLACE_STATUS: &str =
    "EFFECT CHAIN REPLACE  h/l:pane  j/k:移動(自動preview)  /:絞り込み  Space:preview  b:bypass preview  Enter:差し替え  ESC:戻る  ?:help";
