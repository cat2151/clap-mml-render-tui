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
    /// （和音向きの音色は chord 行へ回すため）。カテゴリ設定が空のときは
    /// 「chord 行へ回すべき音色」が定義されていないので何も避けない。
    Free,
    /// drum の bass drum 行。
    Kick,
    /// drum の snare 行。
    Snare,
    /// drum の hi-hat 行。
    HiHat,
    /// drum のその他の打楽器行。カテゴリ内で他の drum 3 役に取られなかった残り全部。
    Percussion,
}

/// 役割と、その役割に対応するカテゴリ一覧・キーワード一覧の組。
///
/// `categories` が空なら「カテゴリでは絞らない」。[`PatchRole::Free`] のときは
/// **避けたい chord 候補のカテゴリ**（＝ chord 用のカテゴリ設定）を渡すこと。
/// その設定が空なら Free は何も避けない（避ける対象が定義されていないため）。
///
/// `keywords` は打楽器の役割でだけ使う。Surge のカテゴリは `Percussion` / `Drums` の
/// 粒度しか無く、kick と hi-hat をカテゴリでは分離できないため、表示名の部分一致で絞る。
/// [`PatchRole::Percussion`] のときは**他の3役ぶんのキーワード**を渡すこと（それに
/// 当たらないものが percussion の候補になる）。
#[derive(Clone, Copy, Debug)]
pub struct RoleFilter<'a> {
    pub role: PatchRole,
    pub categories: &'a [String],
    /// 小文字化済みで渡すこと。照合相手も小文字化した表示名。
    pub keywords: &'a [String],
}

impl<'a> RoleFilter<'a> {
    /// キーワードを見ない役割（chord / bass / arpeggio / free）用。
    pub fn new(role: PatchRole, categories: &'a [String]) -> Self {
        Self {
            role,
            categories,
            keywords: &[],
        }
    }

    pub fn with_keywords(
        role: PatchRole,
        categories: &'a [String],
        keywords: &'a [String],
    ) -> Self {
        Self {
            role,
            categories,
            keywords,
        }
    }
}

/// patch ごとに当てる [`RoleFilter`] を選ぶもの。
///
/// カタログに複数プラグインの音色が並ぶと、絞り込みは 1 組では足りない。Surge の
/// カテゴリを cartridge の音色へ当てると候補が全滅するので（cartridge にカテゴリ階層が
/// 無い）、patch 文字列ごとに引き分ける。
///
/// プラグインが 1 つだけの呼び出し側は [`RoleFilter`] をそのまま渡せばよい
/// （下の実装が全 patch へ同じ絞り込みを返す）。
pub trait RoleFilterLookup {
    fn filter_for(&self, display: &str) -> RoleFilter<'_>;
}

impl RoleFilterLookup for RoleFilter<'_> {
    fn filter_for(&self, _display: &str) -> RoleFilter<'_> {
        *self
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
/// （カテゴリ照合とキーワード照合用）。
///
/// 打楽器の役割は poly 判定を要求しない（drum 行は1音しか鳴らさない）。
pub fn matches_role(
    display: &str,
    normalized_path: &str,
    filter: &RoleFilter<'_>,
    voicing: &dyn VoicingLookup,
) -> bool {
    let in_category = patch_matches_categories(normalized_path, filter.categories);
    let hits_keyword = || {
        filter
            .keywords
            .iter()
            .any(|keyword| normalized_path.contains(keyword.as_str()))
    };
    match filter.role {
        // 未判定（voicing キャッシュが空）も外れ扱いなので、キャッシュが無いと何も当たらない。
        PatchRole::Chord => in_category && voicing.is_poly(display),
        PatchRole::Bass | PatchRole::Arpeggio => in_category,
        // カテゴリが空＝「どれを chord 行へ回すか」が定義されていない。ここで
        // `in_category` を全 true として避けにかかると、poly と判定される patch が
        // 多いプラグイン（全 patch を poly とみなすもの）では候補が 0 件になる。
        PatchRole::Free => {
            filter.categories.is_empty() || !(in_category && voicing.is_poly(display))
        }
        // 打楽器は poly 判定を要求しない（drum 行は1音しか鳴らさない）。
        // キーワードが空ならカテゴリだけで絞る。
        PatchRole::Kick | PatchRole::Snare | PatchRole::HiHat => {
            in_category && (filter.keywords.is_empty() || hits_keyword())
        }
        // percussion は「他の3役に取られなかった残り全部」なので当たり判定が反転する。
        PatchRole::Percussion => in_category && !hits_keyword(),
    }
}

/// 役割の候補になる patch を全部集める。
///
/// 絞り込みは patch ごとに `filters` から引く。プラグインが 1 つだけなら
/// `&RoleFilter` をそのまま渡せる。
pub fn candidates_for_role<'a>(
    pairs: &'a [(String, String)],
    filters: &dyn RoleFilterLookup,
    voicing: &dyn VoicingLookup,
) -> Vec<&'a str> {
    pairs
        .iter()
        .filter(|(display, lower)| {
            matches_role(display, lower, &filters.filter_for(display), voicing)
        })
        .map(|(display, _)| display.as_str())
        .collect()
}

/// 役割の候補から patch を1つ引く。候補が無ければ `None`。
///
/// 当たりが薄いときに引き直しで粘るより、先に候補を絞ったほうが確実で速い。
pub fn pick_for_role(
    pairs: &[(String, String)],
    filters: &dyn RoleFilterLookup,
    voicing: &dyn VoicingLookup,
) -> Option<String> {
    let candidates = candidates_for_role(pairs, filters, voicing);
    let index = random_index(candidates.len())?;
    Some(candidates[index].to_string())
}

fn random_index(len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(rand::rng().random_range(0..len))
}

#[cfg(test)]
mod tests;
