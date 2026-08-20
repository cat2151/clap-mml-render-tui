//! カタログのプラグインごとに、ロード済みの CLAP `PluginEntry` を引く表。
//!
//! # なぜ複数要るか
//! 混在カタログでは「この MML をどのプラグインで鳴らすか」が**音色ごとに変わる**
//! （`docs/adr/0001-patch-string-decides-the-plugin.md`）。in-process のオフラインレンダリングは
//! `PluginEntry` を直接使うので、鳴らしうるプラグインぶんの entry を最初にロードして
//! おき、レンダリングのたびに patch 文字列で引き分ける。
//!
//! # 並び
//! `cmrt_runtime::catalog_plugins(cfg)` と**同じ並び**。先頭が既定プラグイン
//! （＝音色を無指定にした行が鳴るもの）。同じ `Config` から作る限り決まった順になる。

use std::sync::Arc;

use clack_host::prelude::PluginEntry;

/// ロード済み `PluginEntry` への参照をカタログの並びで持つ表。
///
/// 実体は `main()` が所有し、プロセスが終わるまで生きている。ここが持つのは
/// `*const PluginEntry as usize` で、`0` は「その位置のプラグインは in-process では
/// 鳴らせない」を表す（render server backend / テストでは全部 `0`）。
///
/// 参照ではなく `usize` で運ぶ扱いは混在対応より前からのもので、レンダリングを
/// ワーカースレッドへ渡すためにある。安全性の根拠も変えていない（実体が worker より
/// 長生きする）。ここで増えたのは「1 本」が「カタログの並びで N 本」になった点だけ。
#[derive(Clone, Default)]
pub struct PluginEntries {
    ptrs: Arc<[usize]>,
}

impl PluginEntries {
    /// `main()` がロードした entry 列から作る。
    ///
    /// **並びは `catalog_plugins(cfg)` と揃っていること。** ずれると別プラグインの
    /// entry で音色を開こうとして、Surge は 163 byte の SysEx を黙って無視する
    /// （`docs/adr/0009-offline-entry-map.md`）。
    pub fn from_loaded(entries: &[PluginEntry]) -> Self {
        Self {
            ptrs: entries
                .iter()
                .map(|entry| entry as *const PluginEntry as usize)
                .collect(),
        }
    }

    /// in-process レンダリングを使わない経路（render server backend / テスト）用。
    pub fn none() -> Self {
        Self::default()
    }

    /// in-process レンダリングができるか。既定プラグインの entry があるかで判断する。
    pub fn is_available(&self) -> bool {
        self.ptr(0) != 0
    }

    /// カタログ上 `index` 番目のプラグインの entry ポインタ。無ければ `0`。
    pub(crate) fn ptr(&self, index: usize) -> usize {
        self.ptrs.get(index).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests;
