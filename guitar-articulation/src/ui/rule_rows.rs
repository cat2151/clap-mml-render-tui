//! ルールの並び・グループ・キーと、matrix の列ルールの段（[`RULE_LANES`]）。段の中の項目は matrix と奏法リスト overlay のどちらも [`RULE_ROWS`] の順に並ぶ。

use ratatui::style::Color;

use cmrt_tui_core::theme::{MONOKAI_CYAN, MONOKAI_GREEN, MONOKAI_PURPLE, MONOKAI_YELLOW};

use crate::{RowRule, Rule};

use RuleGroup::{AttackRelease, Pitch, Trick};

/// 奏法リスト overlay を開くキー。専用のキーを持たないルールの、matrix の見出しにも付ける。
pub(crate) const RULE_LIST_KEY: char = 't';

/// ルールのまとまり。matrix と overlay の見出しをこの色で塗る。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleGroup {
    /// 行全体のルール（[`ROW_RULE_ROWS`]）。
    Row,
    AttackRelease,
    Pitch,
    Trick,
}

impl RuleGroup {
    pub(crate) fn color(self) -> Color {
        match self {
            RuleGroup::Row => MONOKAI_YELLOW,
            RuleGroup::AttackRelease => MONOKAI_GREEN,
            RuleGroup::Pitch => MONOKAI_CYAN,
            RuleGroup::Trick => MONOKAI_PURPLE,
        }
    }
}

/// 列ルール 1 つの見出し・キー・グループ。
#[derive(Clone, Copy, Debug)]
pub(crate) struct RuleRow {
    pub rule: Rule,
    /// 本画面で切り替えるキー。[`RULE_LIST_KEY`] なら奏法リストからだけ切り替える。
    pub key: char,
    /// 奏法リスト overlay の中で切り替えるキー。
    pub overlay_key: char,
    pub name: &'static str,
    pub group: RuleGroup,
}

impl RuleRow {
    /// 奏法リストの絞り込みで、表示名と並べてマッチさせる別名。
    pub(crate) fn aliases(&self) -> &'static [&'static str] {
        match self.rule {
            // 一般的なギター用語の scrape でも当てる。
            Rule::PickScratch => &["pick scrape"],
            _ => &[],
        }
    }
}

const fn row(
    rule: Rule,
    key: char,
    overlay_key: char,
    name: &'static str,
    group: RuleGroup,
) -> RuleRow {
    RuleRow {
        rule,
        key,
        overlay_key,
        name,
        group,
    }
}

/// 列ルールの並び。グループごとに続けて並べる（アタックとリリース → ピッチ → 飛び道具）。
pub(crate) const RULE_ROWS: [RuleRow; 32] = [
    row(Rule::HammerPull, 'a', 'a', "hammer/pull", AttackRelease),
    row(Rule::PalmMute, 'm', 'm', "palm mute", AttackRelease),
    row(
        Rule::PinchHarmonic,
        'p',
        'p',
        "pinch harmonic",
        AttackRelease,
    ),
    row(
        Rule::NaturalHarmonics,
        RULE_LIST_KEY,
        'b',
        "harmonics",
        AttackRelease,
    ),
    row(Rule::Brushing, RULE_LIST_KEY, 'd', "brush", AttackRelease),
    row(
        Rule::FretMute,
        RULE_LIST_KEY,
        'e',
        "fret mute",
        AttackRelease,
    ),
    row(
        Rule::LongExtra,
        RULE_LIST_KEY,
        'f',
        "long/extra",
        AttackRelease,
    ),
    row(
        Rule::PowerChord,
        RULE_LIST_KEY,
        'i',
        "power chord",
        AttackRelease,
    ),
    row(
        Rule::PositionRelease,
        RULE_LIST_KEY,
        'n',
        "position rel",
        AttackRelease,
    ),
    row(Rule::Slide, '/', 'o', "slide up/down", Pitch),
    row(Rule::Choke, 'c', 'c', "bend", Pitch),
    row(Rule::Vibrato, 'v', 'v', "vibrato", Pitch),
    row(Rule::SlideIn, RULE_LIST_KEY, 'q', "slide in", Pitch),
    row(Rule::SlideOut, RULE_LIST_KEY, 'r', "slide out", Pitch),
    row(
        Rule::AutoSlideOut,
        RULE_LIST_KEY,
        's',
        "auto slide out",
        Pitch,
    ),
    row(
        Rule::PseudoLegato,
        RULE_LIST_KEY,
        't',
        "pseudo legato",
        Pitch,
    ),
    row(Rule::Portamento, RULE_LIST_KEY, 'u', "portamento", Pitch),
    row(Rule::TrillHalf, RULE_LIST_KEY, 'w', "trill half", Pitch),
    row(Rule::TrillWhole, RULE_LIST_KEY, 'x', "trill whole", Pitch),
    row(
        Rule::TrillMinorThird,
        RULE_LIST_KEY,
        'y',
        "trill min3",
        Pitch,
    ),
    row(
        Rule::TrillMajorThird,
        RULE_LIST_KEY,
        'z',
        "trill maj3",
        Pitch,
    ),
    row(
        Rule::UnisonBendAuto,
        RULE_LIST_KEY,
        'A',
        "unison bend",
        Pitch,
    ),
    row(
        Rule::UnisonBendManual,
        RULE_LIST_KEY,
        'B',
        "unison manual",
        Pitch,
    ),
    row(Rule::PickScratch, 'g', 'g', "pick scratch", Trick),
    row(
        Rule::ChromaticRun,
        RULE_LIST_KEY,
        'C',
        "chromatic run",
        Trick,
    ),
    row(
        Rule::SlideFxDown,
        RULE_LIST_KEY,
        'D',
        "slide fx down",
        Trick,
    ),
    row(Rule::SlideFxUp, RULE_LIST_KEY, 'E', "slide fx up", Trick),
    row(Rule::SlideFxWow, RULE_LIST_KEY, 'F', "slide fx wow", Trick),
    row(Rule::EffectHello, RULE_LIST_KEY, 'G', "fx hello", Trick),
    row(
        Rule::EffectResonance,
        RULE_LIST_KEY,
        'H',
        "fx resonance",
        Trick,
    ),
    row(
        Rule::EffectSlideNoise,
        RULE_LIST_KEY,
        'I',
        "fx slide noise",
        Trick,
    ),
    row(
        Rule::EffectHardStop,
        RULE_LIST_KEY,
        'J',
        "fx hard stop",
        Trick,
    ),
];

/// matrix の列ルールの段 1 つ。同じ列で同時に ON にできるルールだけを別の段に分ける。
/// 段の所属は [`Rule::is_exclusive`] / [`Rule::selects_release_shape`] から導き、段ごとのルール表を別に持たない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleLane {
    /// KS を決めるルール。グループを問わず互いに排他。
    KeySwitch,
    /// 誰とも排他でないルール 1 つ。
    Single(Rule),
    /// 離し音の形を決めるルール。互いに排他。
    ReleaseShape,
}

/// matrix の列ルールの段の並び。
pub(crate) const RULE_LANES: [RuleLane; 5] = [
    RuleLane::KeySwitch,
    RuleLane::Single(Rule::Vibrato),
    RuleLane::Single(Rule::LongExtra),
    RuleLane::Single(Rule::PowerChord),
    RuleLane::ReleaseShape,
];

impl RuleLane {
    pub(crate) fn of(rule: Rule) -> RuleLane {
        if rule.is_exclusive() {
            RuleLane::KeySwitch
        } else if rule.selects_release_shape() {
            RuleLane::ReleaseShape
        } else {
            RuleLane::Single(rule)
        }
    }

    /// 段の中のルール（[`RULE_ROWS`] の順）。
    pub(crate) fn rule_rows(self) -> impl Iterator<Item = &'static RuleRow> {
        RULE_ROWS
            .iter()
            .filter(move |rule_row| RuleLane::of(rule_row.rule) == self)
    }

    /// 複数のルールが入る段で、どれも ON でないときの見出しの名前。1 つしか入らない段は `None`。
    pub(crate) fn idle_name(self) -> Option<&'static str> {
        match self {
            RuleLane::KeySwitch => Some("KS"),
            RuleLane::Single(_) => None,
            RuleLane::ReleaseShape => Some("release"),
        }
    }
}

/// 行全体で ON/OFF するルールの段の見出しとトグルのキー。列ルールの段の上に並べ、[`RuleGroup::Row`] の色で塗る。
/// 汚し 2 つは並べる。自動 H/P は H/P の段のすぐ上に置く（どちらも列の H/P を示す）。
pub(crate) const ROW_RULE_ROWS: [(RowRule, char, &str); 4] = [
    (RowRule::Humanize, 'd', "humanize"),
    (RowRule::HumanizeRelease, 'r', "humanize release"),
    (RowRule::EconomyPicking, 'e', "economy picking"),
    (RowRule::AutoHammerPull, 's', "auto hammer/pull"),
];
