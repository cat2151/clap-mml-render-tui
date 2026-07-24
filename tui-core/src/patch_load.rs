//! パッチ一覧のバックグラウンド読み込み状態（画面横断で共有）。
//!
//! 起動時の同期スキャンによる遅延を避けるため、パッチ一覧は別スレッドで収集する。
//! その途中経過を notepad（音色選択）と keyboard（patch catalog）の双方が読むため、
//! 状態型だけをここに置く。スレッドの起動は所有者（app）が行う。

/// バックグラウンドパッチ読み込みの状態。
pub enum PatchLoadState {
    Loading,
    /// (表示名, 小文字化済み表示名) のペア。
    Ready(Vec<(String, String)>),
    Err(String),
}
