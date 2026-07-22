//! ファイルに永続化される loop browser の状態を、値・保存先・可書き込み・エラーの
//! 4 点セットとしてまとめて扱うためのラッパ。
//!
//! metadata / random decks のように「読み込み → 値を変更 → 保存、失敗時はロールバック」
//! という同型の手続きが複数箇所へコピーされていたのを 1 箇所へ集約する。
use std::path::{Path, PathBuf};

/// 永続化される単一ドキュメント。
pub(crate) struct PersistedDoc<T> {
    /// メモリ上の現在値。
    pub(crate) value: T,
    /// 保存先パス。解決できなかった場合は `None`。
    pub(crate) path: Option<PathBuf>,
    /// 書き込み可能かどうか（読み込みに失敗した場合などは `false`）。
    pub(crate) writable: bool,
    /// 直近の読み書きで発生したエラー文言。
    pub(crate) error: Option<String>,
}

impl<T> PersistedDoc<T> {
    /// まだファイルへ紐付いていない、書き込み可能な初期値を作る（`Default` 相当）。
    pub(crate) fn in_memory(value: T) -> Self {
        Self {
            value,
            path: None,
            writable: true,
            error: None,
        }
    }

    /// 保存先を解決し読み込む。解決・読み込みに失敗した場合は `default` 値を保持し、
    /// `writable = false` とエラー文言を残す。
    pub(crate) fn load(
        resolve: impl FnOnce() -> anyhow::Result<PathBuf>,
        read: impl FnOnce(&Path) -> anyhow::Result<T>,
        default: impl FnOnce() -> T,
    ) -> Self {
        match resolve() {
            Ok(path) => match read(&path) {
                Ok(value) => Self {
                    value,
                    path: Some(path),
                    writable: true,
                    error: None,
                },
                Err(error) => Self {
                    value: default(),
                    path: Some(path),
                    writable: false,
                    error: Some(error.to_string()),
                },
            },
            Err(error) => Self {
                value: default(),
                path: None,
                writable: false,
                error: Some(error.to_string()),
            },
        }
    }
}

impl<T: Clone> PersistedDoc<T> {
    /// 値を `mutate` で変更し `save` で永続化する。保存に失敗したら値を元へ戻して
    /// エラー文言を記録し `None` を返す。成功時はエラーを消して `mutate` の戻り値を返す。
    ///
    /// `save` には現在のパスを渡す（`path` が `None` の場合は保存をスキップし成功扱い）。
    /// 呼び出し側は必要に応じて事前に `writable` を確認すること。
    pub(crate) fn try_mutate<R>(
        &mut self,
        mutate: impl FnOnce(&mut T) -> R,
        save: impl FnOnce(&Path, &T) -> anyhow::Result<()>,
        err_prefix: &str,
    ) -> Option<R> {
        let previous = self.value.clone();
        let result = mutate(&mut self.value);
        if let Some(path) = self.path.clone() {
            if let Err(error) = save(&path, &self.value) {
                self.value = previous;
                self.error = Some(format!("{err_prefix}: {error}"));
                return None;
            }
        }
        self.error = None;
        Some(result)
    }
}
