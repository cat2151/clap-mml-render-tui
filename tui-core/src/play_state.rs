//! 再生/レンダリングの進行状態。
//!
//! ステータスバー表示のために画面横断（notepad / loop browser）で共有される。

#[derive(Clone, PartialEq)]
pub enum PlayState {
    Idle,
    Running(String), // レンダリング中
    Playing(String), // 演奏中
    Done(String),
    Err(String),
}
