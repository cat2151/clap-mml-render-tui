//! 演奏ループのテスト。
//!
//! - [`cues`] … 何を鳴らすか（実サーバーもファイルも要らない）
//! - [`measure_log`] … 小節 1 行のログの読み方
//! - [`play_loop`] … 実サーバーへ繋いでループを丸ごと走らせる（通常は skip）
//! - [`state_load`] … 小節ごとの state load の実測（通常は skip）
//! - [`timeline`] … 発音位置のグリッド（実サーバー不要）
//! - [`jitter`] … 実サーバーで発音位置のジッタが 0 であること（通常は skip）
//! - [`cache_lookup`] … セル編集でキャッシュが消えること
//! - [`slot_headroom`] … クロックが先行しても正しい小節が鳴ること（実サーバー不要）
//! - [`capture`] … 実キャッシュで鳴らして混ざった出力を録る（通常は skip）

mod cache_lookup;
mod capture;
mod cues;
mod jitter;
mod measure_log;
mod play_loop;
mod slot_headroom;
mod state_load;
mod timeline;

use std::path::PathBuf;

/// テスト用のキャッシュ WAV パス。実ファイルは要らない
/// （`measure_live_cues` は「存在するか」を呼び出し側の責務にしてある）。
fn wav(row: usize) -> PathBuf {
    PathBuf::from(format!("C:/daw_cache/track{row}_meas1.wav"))
}
