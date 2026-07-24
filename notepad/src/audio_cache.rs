use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

/// notepad（メイン）画面の音声キャッシュ3点セット。
///
/// 「MML→レンダリング済みサンプル」の LRU 本体（`cache`）と、その退避順序
/// （`order`）、ディスクキャッシュ上に存在する有効 wav のハッシュ集合
/// （`known_disk_hashes`）は常に整合させて更新する必要がある（cache へ入れたら
/// order にも積む、`flush_notepad_disk_cache` 後に known_disk_hashes を貼り直す等）。
/// これら3つの所有を1型へ閉じ込め、手動同期の対象を局所化する。
pub(crate) struct NotepadAudioCache {
    /// MML文字列 → レンダリング済みサンプルのキャッシュ
    pub(crate) cache: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    /// `cache` の退避順序（先頭が最古）。上限超過時に先頭から追い出す。
    pub(crate) order: Arc<Mutex<VecDeque<String>>>,
    /// notepad ディスクキャッシュディレクトリに現在存在する、有効な wav の MML ハッシュ集合。
    /// `cache` から追い出された行のディスクフォールバック判定と、一覧UIでの
    /// 即再生可能マーク表示に使う。起動時と `flush_notepad_disk_cache` 実行後に更新する。
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
