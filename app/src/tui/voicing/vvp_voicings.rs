//! `.vvp`（Vaporizer2）の mono/poly を、音色ファイルの先頭から読む層。
//!
//! # なぜ probe でも共有 JSON でもないか
//! Vaporizer2 の note dialect は **MIDI だけ**で、CLAP note event の `note_id` に対する
//! NOTE_END が返らない（play server repo の `docs/adr/0001-measured-plugin-capabilities.md`）。
//! keyboard の実行時 probe は NOTE_END を数える方式なので「mono」と「測れなかった」を
//! 区別できず、Vaporizer2 では使えない。Surge 専用の共有 JSON も、キーが Surge の
//! patch 表示パスなので 1 件も当たらない。
//!
//! 一方 `.vvp` は XML で、`m_uPolyMode` が**ファイルの先頭に書いてある**。読めば済む。
//!
//! # 読み方
//! 実体は play server repo 側の `read_vvp_header`（先頭 4096 バイトだけ読む。460 ファイル
//! 全読み = 681MB は絶対にしない）。ここはその結果を display 文字列で引ける形に memo する。
//!
//! memo が要るのは、用途別の候補を数える `candidates_for_role` が**一覧全体を舐める**ため。
//! TUI は明示的CLIが永続化した判定結果から memo を復元する。音色ファイルを直接読むのは
//! cache構築と、cacheを使わない診断・テストだけ。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use cmrt_tui_core::patch_plugins::{CatalogPlugin, PatchPlugins};

use crate::realtime_play::PatchVoicing;

/// display 文字列 → mono/poly の memo。
///
/// 値の `None` は「読みに行ったが読めなかった」。**まだ読んでいない**（＝キーが無い）と
/// 区別するために `Option` を値へ入れてある。区別しないと、壊れた 1 ファイルを
/// フィルタのたびに開き直すことになる。
///
/// cacheの読み込みはバックグラウンドスレッドなので、memo は `Arc` で共有する。
#[derive(Clone, Default)]
pub(in crate::tui) struct VvpVoicings {
    memo: Arc<Mutex<HashMap<String, Option<PatchVoicing>>>>,
}

impl VvpVoicings {
    /// file cacheから復元した判定をmemoへ入れる。`Unknown`は判定不能として記憶する。
    pub(in crate::tui) fn load_persisted(
        &self,
        entries: impl IntoIterator<Item = (String, PatchVoicing)>,
    ) -> usize {
        let mut memo = self.memo.lock().unwrap();
        memo.clear();
        memo.extend(entries.into_iter().map(|(patch, voicing)| {
            let decided = match voicing {
                PatchVoicing::Mono | PatchVoicing::Poly => Some(voicing),
                PatchVoicing::Unknown => None,
            };
            (patch, decided)
        }));
        memo.len()
    }

    /// この `.vvp` の mono/poly。読めなければ `None`（＝未判定扱い）。
    ///
    /// `plugin` は display 文字列を実ファイルへ戻すための基点として使う。
    pub(in crate::tui) fn voicing(
        &self,
        plugin: &CatalogPlugin,
        patch: &str,
    ) -> Option<PatchVoicing> {
        if let Some(memoized) = self.memo.lock().unwrap().get(patch) {
            return *memoized;
        }
        let decided = read_voicing(plugin, patch);
        self.memo.lock().unwrap().insert(patch.to_string(), decided);
        decided
    }

    /// 一覧に載っている `.vvp` を先に全部読んでおく。診断・テスト用。
    ///
    /// **`.vvp` 以外は 1 バイトも読まない。** 引き分けは
    /// [`PatchPlugins::index_for_patch`] ＝ PATCH 欄の wheel と同じ述語を通すので、
    /// 「一覧では Vaporizer2 の音色なのに、ここでは別プラグインの基点で開こうとする」が
    /// 起きない。
    pub(in crate::tui) fn prefetch(
        &self,
        plugins: &PatchPlugins,
        pairs: &[(String, String)],
    ) -> usize {
        let mut read = 0usize;
        for (display, _) in pairs {
            if !cmrt_core::is_vvp_patch_path(display) {
                continue;
            }
            self.voicing(plugins.for_patch(display), display);
            read += 1;
        }
        read
    }
}

fn read_voicing(plugin: &CatalogPlugin, patch: &str) -> Option<PatchVoicing> {
    let path = patch_file_path(plugin, patch);
    match cmrt_core::read_vvp_header(&path) {
        Ok(header) => Some(if header.poly {
            PatchVoicing::Poly
        } else {
            PatchVoicing::Mono
        }),
        Err(error) => {
            // 読めなかった 1 件で画面が止まっては困るので、未判定として続ける。
            // 未判定の音色は和音行の候補から外れるだけ（`matches_role`）。
            crate::logging::global_log_sink(&format!(
                "vvp-voicing: event=read-failed patch=\"{patch}\" error=\"{error:#}\""
            ));
            None
        }
    }
}

/// display 文字列を実ファイルへ戻す。
///
/// display は `collect_patch_pairs` が `to_relative(base, path)` で作ったもので、
/// 区切りは `/`。`Path::join` は Windows でも `/` を受けるのでそのまま繋げる。
/// 基点が取れなかったプラグイン（`base` が `None`）の display は絶対パスそのもの。
fn patch_file_path(plugin: &CatalogPlugin, patch: &str) -> PathBuf {
    match plugin.base.as_deref() {
        Some(base) => Path::new(base).join(patch),
        None => PathBuf::from(patch),
    }
}

#[cfg(test)]
mod tests;
