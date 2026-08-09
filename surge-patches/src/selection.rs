//! 用途（役割）別の候補判定と抽選。
//!
//! 「この行にはどの patch なら成立するか」を1本の API へ畳んである。判定と抽選が
//! 同じ述語を共有するので、ランダム適用と一覧フィルタで候補集合がずれない。

use rand::RngExt;

use crate::layout::patch_matches_categories;

/// patch の用途。行ごとに成立条件が違う。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchRole {
    /// 和音を1 instance へ重ねる行。mono patch では最後の1音しか鳴らないので poly 限定。
    Chord,
    /// 単音の bass 行。和音と違い poly 判定は要らない。
    Bass,
    /// 声部を順に鳴らすアルペジオ行。同時発音しないので poly 判定は要らない。
    Arpeggio,
    /// 用途が決まっていない行。[`PatchRole::Chord`] の候補になる patch を避ける
    /// （和音向きの音色は chord 行へ回すため）。
    Free,
}

/// 役割と、その役割に対応するカテゴリ一覧の組。
///
/// `categories` が空なら「カテゴリでは絞らない」。[`PatchRole::Free`] のときは
/// **避けたい chord 候補のカテゴリ**（＝ chord 用のカテゴリ設定）を渡すこと。
#[derive(Clone, Copy, Debug)]
pub struct RoleFilter<'a> {
    pub role: PatchRole,
    pub categories: &'a [String],
}

impl<'a> RoleFilter<'a> {
    pub fn new(role: PatchRole, categories: &'a [String]) -> Self {
        Self { role, categories }
    }
}

/// patch の mono/poly 判定。判定データを持つのは呼び出し側（voicing キャッシュ）なので、
/// ここでは trait で受け取るだけにする。
pub trait VoicingLookup {
    /// poly と**判明している**場合だけ true。未判定は false 扱い。
    fn is_poly(&self, patch: &str) -> bool;
}

/// patch が役割の候補になるか。
///
/// `display` は voicing 判定のキー、`normalized_path` は小文字化済みのパス
/// （カテゴリ照合用）。
pub fn matches_role(
    display: &str,
    normalized_path: &str,
    filter: &RoleFilter<'_>,
    voicing: &dyn VoicingLookup,
) -> bool {
    let in_category = patch_matches_categories(normalized_path, filter.categories);
    match filter.role {
        // 未判定（voicing キャッシュが空）も外れ扱いなので、キャッシュが無いと何も当たらない。
        PatchRole::Chord => in_category && voicing.is_poly(display),
        PatchRole::Bass | PatchRole::Arpeggio => in_category,
        PatchRole::Free => !(in_category && voicing.is_poly(display)),
    }
}

/// 役割の候補から patch を1つ引く。候補が無ければ `None`。
///
/// 当たりが薄いときに引き直しで粘るより、先に候補を絞ったほうが確実で速い。
pub fn pick_for_role(
    pairs: &[(String, String)],
    filter: &RoleFilter<'_>,
    voicing: &dyn VoicingLookup,
) -> Option<String> {
    let candidates = pairs
        .iter()
        .filter(|(display, lower)| matches_role(display, lower, filter, voicing))
        .collect::<Vec<_>>();
    let index = random_index(candidates.len())?;
    Some(candidates[index].0.clone())
}

fn random_index(len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(rand::rng().random_range(0..len))
}

#[cfg(test)]
mod tests;
