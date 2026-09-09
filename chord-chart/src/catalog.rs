//! コード進行カタログの供給元と、そこからの抽選。
//!
//! カタログの取得・キャッシュ（ネットワーク）は app の責務なので、この crate へは
//! 「一覧を返す関数」だけを借りる。**遅延評価**であることが前提: キャッシュがまだ無い
//! 初回は `progressions()` の中でネットワーク取得の完了を待つ（最大 20 秒）。
//! そのため**画面を開いた瞬間ではなく、`g` / `r` を押した瞬間に初めて呼ぶ**。

use std::fmt;
use std::sync::Arc;

/// カタログがまだ手元に無いとき（未注入 / 取得できていない）の理由。
///
/// **無反応で終わらせない**ための文言。押しても何も起きないと、キーが効いていないのか
/// データが無いのか区別が付かない。
pub(crate) const NO_CATALOG_MESSAGE: &str = "コード進行データがありません";

/// コード進行の一覧を返す関数。app 側が注入する。
#[derive(Clone)]
pub struct ChordProgressionSource(Arc<dyn Fn() -> Vec<String> + Send + Sync>);

impl ChordProgressionSource {
    pub fn new(source: Arc<dyn Fn() -> Vec<String> + Send + Sync>) -> Self {
        Self(source)
    }

    /// カタログを引く。**ここでブロックし得る**（上のモジュールコメント）。
    pub fn progressions(&self) -> Vec<String> {
        (self.0)()
    }
}

/// `ChordChartScreen` が `#[derive(Debug)]` のままでいられるように手で書く。
/// 関数は中身を出せないので、持っていることだけを示す。
impl fmt::Debug for ChordProgressionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChordProgressionSource(..)")
    }
}

/// カタログから進行を 1 つ引き、**そのまま**返す。
///
/// 引いたものを検証しないのは、この画面が degrees を一切解釈しないから
/// （書式は chord2mml-rs のものであり、読めるかどうかは演奏側の責任＝別スコープ）。
/// 検証を挟むと、この crate が進行のパーサへ依存し直すことになる。
pub(crate) fn pick_progression(source: Option<&ChordProgressionSource>) -> Result<String, String> {
    let Some(source) = source else {
        return Err(NO_CATALOG_MESSAGE.to_string());
    };
    let progressions = source.progressions();
    // 空カタログはここで弾く。`random_index` が `None` を返すのも len == 0 のときだけ。
    let Some(index) = cmrt_tui_core::random::random_index(progressions.len()) else {
        return Err(NO_CATALOG_MESSAGE.to_string());
    };
    Ok(progressions[index].clone())
}

#[cfg(test)]
mod tests;
