//! patch ごとの mono/poly 判定結果を voicing_cache.json に保存する。
//!
//! 判定 (`/live-patch-probe`) は patch を切り替えるたびに実プラグインを走らせるため
//! 待ち時間が発生する。同じ patch の判定結果は変わらないので、一度判定したものを
//! ファイルにキャッシュし、以降は判定を省いて `/live-patch` だけを呼ぶ。
//!
//! キーは patch の相対パス文字列を正規化したもの。fxp を差し替えた場合は
//! キャッシュを消して作り直す想定で、mtime やハッシュによる無効化は行わない。

use std::collections::HashMap;

use anyhow::Result;

use crate::realtime_play::PatchVoicing;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct VoicingCache {
    #[serde(default)]
    pub(crate) entries: HashMap<String, PatchVoicing>,
}

impl VoicingCache {
    pub(crate) fn get(&self, patch: &str) -> Option<PatchVoicing> {
        self.entries
            .get(&crate::patches::normalize_patch_lookup_key(patch))
            .copied()
    }

    /// 判定結果を登録する。内容が変化したときだけ `true` を返す。
    /// `Unknown` は判定できなかったことを意味するのでキャッシュしない。
    pub(crate) fn insert(&mut self, patch: &str, voicing: PatchVoicing) -> bool {
        if voicing == PatchVoicing::Unknown {
            return false;
        }
        let key = crate::patches::normalize_patch_lookup_key(patch);
        if key.is_empty() {
            return false;
        }
        if self.entries.get(&key) == Some(&voicing) {
            return false;
        }
        self.entries.insert(key, voicing);
        true
    }

    pub(crate) fn from_shared_json(text: &str) -> Result<Self> {
        let parsed: Self = serde_json::from_str(text)?;
        let mut normalized = Self::default();
        for (patch, voicing) in parsed.entries {
            if voicing == PatchVoicing::Unknown {
                anyhow::bail!("共有voicing判定にunknownは指定できません: {patch}");
            }
            let key = crate::patches::normalize_patch_lookup_key(&patch);
            if key.is_empty() {
                anyhow::bail!("共有voicing判定に空のpatch keyがあります");
            }
            if let Some(previous) = normalized.entries.insert(key.clone(), voicing) {
                if previous != voicing {
                    anyhow::bail!("共有voicing判定の正規化後keyが重複しています: {key}");
                }
            }
        }
        Ok(normalized)
    }
}

pub(crate) fn save_voicing_cache(cache: &VoicingCache) -> Result<()> {
    let Some(path) = super::paths::voicing_cache_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(cache)?;
    std::fs::write(&path, json)?;
    Ok(())
}

pub(crate) fn load_voicing_cache() -> VoicingCache {
    let Some(path) = super::paths::voicing_cache_path() else {
        return VoicingCache::default();
    };
    if !path.exists() {
        return VoicingCache::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
