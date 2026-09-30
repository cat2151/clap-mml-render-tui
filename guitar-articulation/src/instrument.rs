//! 画面が鳴らす METAL-GTX の版。
//!
//! 入った直後は読み込みの軽い Lite（[`PATCH`]）で鳴らし、裏で Full（[`FULL_PATCH`]）を
//! 読み終えたら次の再生から Full にする。どの段階かは host が音源の状態から決めて
//! [`crate::GuitarArticulationScreen::set_instrument`] で渡す。画面は見出しに出すだけ。
//! [`StartupInstrument::Full`] のときは Lite を飛ばし、入った時点で Full を読む。

use serde::{Deserialize, Serialize};

use crate::PATCH;

/// Full 版の音色（sforzando の `patches_dirs` からの相対）。KS と CC の割り当ては [`PATCH`] と同じ。
pub const FULL_PATCH: &str = "sfz/UI_METAL-GTX/Programs/01-METAL-GTX Full.sfz";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Instrument {
    /// Lite で鳴らす。Full は読んでいない。
    #[default]
    Lite,
    /// Lite で鳴らし、裏で Full を読んでいる。
    LiteLoadingFull,
    /// Full を読み終えた。以後は Full で鳴らす。
    Full,
}

impl Instrument {
    /// この段階で鳴らす音色。
    pub fn patch(self) -> &'static str {
        match self {
            Instrument::Lite | Instrument::LiteLoadingFull => PATCH,
            Instrument::Full => FULL_PATCH,
        }
    }

    /// 見出しに出す名前。
    pub fn label(self) -> &'static str {
        match self {
            Instrument::Lite => "METAL-GTX Lite",
            Instrument::LiteLoadingFull => "METAL-GTX Lite（Full 読み込み中）",
            Instrument::Full => "METAL-GTX Full",
        }
    }
}

/// 画面に入ったとき最初に読む版。`f` で切り替え、設定ファイルに残す。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupInstrument {
    /// Lite を読んで鳴らし、裏で Full を先読みする。
    #[default]
    LiteThenFull,
    /// 最初から Full を読む（読み終えるまで待つ）。
    Full,
}

impl StartupInstrument {
    pub fn toggled(self) -> Self {
        match self {
            StartupInstrument::LiteThenFull => StartupInstrument::Full,
            StartupInstrument::Full => StartupInstrument::LiteThenFull,
        }
    }

    /// 見出しとログに出す名前。
    pub fn label(self) -> &'static str {
        match self {
            StartupInstrument::LiteThenFull => "Lite→Full",
            StartupInstrument::Full => "Full",
        }
    }
}
