use super::*;

// フッタは 1 行に全 keybind を並べ、入りきらない分は**折り返さず切り捨てる**。
// そのため、後ろのほうの keybind を探すテストは実描画の幅をここで確保する。
// キーを 1 つ足すたびに必要な幅が伸びるので、落ちたら「フッタが伸びた」合図。
// （全部を読む手段はヘルプ `K` 側にあるので、実機の端末幅がここに届く必要は無い。）

/// `a:A-B`（フッタの中ほど）まで見える幅。実測 166 桁 + 枠 2 桁。
const FOOTER_WIDE_TEST_WIDTH: u16 = 172;
/// フッタの全 keybind が見える幅。DAW のフッタは実測 282 桁 + 枠 2 桁。
const FOOTER_FULL_KEYBIND_TEST_WIDTH: u16 = 284;

mod footer;
mod grid;
mod grid_chord_row;
mod help;
mod layout;
mod logs;
mod mixer;
mod solo_mute;
