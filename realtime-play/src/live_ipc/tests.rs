//! `live_ipc` のテスト。責務ごとに分けてある。
//!
//! - `amplitude`: dB 変換の純粋な計算
//! - `harness`: 実サーバーを起こして stderr を読む道具
//! - `standby_preload`: 先読みの実サーバーテスト（`#[ignore]`）
//! - `grid_cycle`: grid の 1 周ぶんを TUI 無しで流し、metrics の退行を見る（`#[ignore]`）
//! - `bank_switch`: 切替後の bank が実際に音を出すかを auto gain で見る（`#[ignore]`）
//! - `timeline_during_preload`: 先読みのロード中も timeline の供給が止まらないこと（`#[ignore]`）

mod amplitude;
mod bank_switch;
mod grid_cycle;
mod harness;
mod standby_preload;
mod timeline_during_preload;
