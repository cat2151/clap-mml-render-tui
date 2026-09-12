use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

/// notepad（メイン）画面の音声キャッシュ 3 点セット。
///
/// LRU 本体（`cache`）・その退避順序（`order`）・ディスク上の有効 wav のハッシュ集合
/// （`known_disk_hashes`）は常に整合させる（cache へ入れたら order にも積む、
/// `flush_notepad_disk_cache` 後に known_disk_hashes を貼り直す）。所有を 1 型へ閉じて手動同期の対象を局所化する。
pub(crate) struct NotepadAudioCache {
    /// MML 文字列 → レンダリング済みサンプル。
    pub(crate) cache: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    /// `cache` の退避順序（先頭が最古）。上限超過時に先頭から追い出す。
    pub(crate) order: Arc<Mutex<VecDeque<String>>>,
    /// ディスクキャッシュに現在存在する有効な wav の MML ハッシュ集合。追い出された行の
    /// ディスクフォールバック判定と、一覧 UI の即再生可能マークに使う。
    pub(crate) known_disk_hashes: Arc<Mutex<HashSet<u64>>>,
}

impl NotepadAudioCache {
    pub(crate) fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            order: Arc::new(Mutex::new(VecDeque::new())),
            known_disk_hashes: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}
