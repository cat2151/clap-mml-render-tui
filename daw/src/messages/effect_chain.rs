//! EFFECT CHAIN overlay（`x`）でユーザーへ直接表示する文言のうち、DAW だけのもの。
//! overlay 自体の文言は `cmrt_effect_chain_select::messages`。

pub(crate) const INSTRUMENT_LABEL: &str = "instrument: ";
pub(crate) const PLAYABLE_TRACK_ONLY: &str = "effect chain は演奏 track でのみ使用できます";

pub(crate) const STATUS: &str =
    "EFFECT CHAIN  j/k:移動  a:追加  r:差替  dd:削除  b:bypass  Alt+↑↓:並替(自動preview)  Space:preview  Enter:確定  ESC:破棄  ?:help";
pub(crate) const ADD_STATUS: &str =
    "EFFECT CHAIN ADD  h/l:pane  j/k:移動(自動preview)  /:絞り込み  Space:preview  b:bypass  Enter:末尾へ追加  r:random  ESC:戻る  ?:help";
pub(crate) const REPLACE_STATUS: &str =
    "EFFECT CHAIN REPLACE  h/l:pane  j/k:移動(自動preview)  /:絞り込み  Space:preview  b:bypass  Enter:差し替え  r:random  ESC:戻る  ?:help";
