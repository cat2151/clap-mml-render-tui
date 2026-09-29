//! effect chain overlay でユーザーへ直接表示する文言。

pub const OVERLAY_TITLE: &str = " EFFECT CHAIN ";
pub const ADD_OVERLAY_TITLE: &str = " EFFECT CHAIN: add preset ";
pub const REPLACE_OVERLAY_TITLE: &str = " EFFECT CHAIN: replace preset ";

pub const EMPTY_CHAIN: &str = "(effect なし。a で追加)";
pub const NO_PRESETS: &str = "effect plugin が見つからない";
pub const NOT_AVAILABLE_ON_THIS_BACKEND: &str = "effect を持たない経路では使えない";

pub const ADD_QUERY_TITLE: &str = " query ";
pub const ADD_QUERY_TITLE_EDITING: &str = " query (Enter=確定 / ESC=中断) ";
pub const ADD_QUERY_PLACEHOLDER: &str = "/ で list を絞り込み";

pub const FOOTER: &str =
    "j/k:移動  PgUp/PgDn/Home/End:大移動  a:追加  r:差替  dd:削除  b:bypass  Alt+↑↓:並替(いずれも自動preview)  Space:preview  Enter:確定  ESC:破棄  ?:help";
pub const ADD_FOOTER: &str =
    "h/l:pane  j/k:移動(自動preview)  /:絞り込み  PgUp/PgDn/Home/End:大移動  r:random  Space:preview  b:bypass preview  Enter:末尾へ追加  ESC:戻る  ?:help";
pub const REPLACE_FOOTER: &str =
    "h/l:pane  j/k:移動(自動preview)  /:絞り込み  PgUp/PgDn/Home/End:大移動  r:random  Space:preview  b:bypass preview  Enter:差し替え  ESC:戻る  ?:help";
