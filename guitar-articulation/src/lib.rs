//! rawの MIDI イベント列を、METAL-GTX（sfz）のキースイッチ入りの列へ変換する。
//!
//! 変換は 1 ルール = 1 パスの純関数を順に積む（[`convert`] がその並び）。
//! KS 番号は METAL-GTX 固有なので、汎用の `cmrt-midi-filter` には置かない。
//! 変換を試す画面（[`GuitarArticulationScreen`] と [`ui`]）も同じ crate に置く。

mod auto_pick;
mod column_map;
mod column_rule_anchor;
mod control;
mod glide;
mod hammer_pull;
mod history;
mod humanize;
mod instrument;
mod keyswitch;
mod notes;
mod picking;
mod release;
mod report;
mod sample_midi;
mod scratch;
mod screen;
mod settings;
mod single_note;
mod strings;
pub mod ui;
mod voicing;

#[cfg(test)]
mod test_effects;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub use auto_pick::{auto_pick_columns, RUN_MIN_NOTES, RUN_PICK_EVERY};
pub use cmrt_midi_filter::TimedMidiEvent;
pub use column_rule_anchor::ColumnRuleAnchor;
pub use control::{
    control_events, SLIDE_WIDTH_CC, SLIDE_WIDTH_CC_DEFAULT, VIBRATO_DEPTH, VIBRATO_DEPTH_CC,
};
pub use glide::{apply_glide_rules, slide_semitones, BEND_PITCHES, SLIDE_MAX_SEMITONES};
pub use hammer_pull::apply_hammer_pull;
pub use history::{
    load_history, save_history, GuitarArticulationHistory, GuitarArticulationHistoryEntry,
    HISTORY_MAX_LEN,
};
pub use instrument::{Instrument, StartupInstrument, FULL_PATCH};
pub use keyswitch::{keyswitch_events, KEYSWITCH_RESET_SECONDS, KEYSWITCH_VELOCITY};
pub use notes::{notes_from_events, Note};
pub use picking::{articulate, Articulated, UNACCENTED_PICK_VELOCITY_PERCENT};
pub use report::{compare, report};
pub use sample_midi::{is_keyswitch_pitch, keyswitch_name, SampleMidi, SAMPLE_MIDI_DIR};
pub use scratch::{apply_pick_scratch, pick_scratch_pitch, PICK_SCRATCH_PITCHES};
pub use screen::{GuitarArticulationAction, GuitarArticulationScreen, Take, DEFAULT_MML};
pub use settings::{load_settings, save_settings, GuitarArticulationSettings};
pub use single_note::column_events;
pub use strings::{picks_string, strings_by_column, REACH_SEMITONES};
pub use voicing::{apply_voicing_rules, MUTE_PITCHES, PINCH_HARMONIC_PITCHES};

/// 画面に入ったときに読む音色（sforzando の `patches_dirs` からの相対）。KS 番号が METAL-GTX に固有なので固定する。
pub const PATCH: &str = "sfz/UI_METAL-GTX/Programs/02-METAL-GTX Lite.sfz";

/// 音ごとの奏法。METAL-GTX の KS（`sw_last` のラッチ式）1 つに対応する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Articulation {
    /// sfz の `sw_default`。ピッキングした音。
    SusDown,
    /// アップストロークでピッキングした音。
    SusUp,
    HammerOn,
    PullOff,
    /// パームミュートしてダウンストロークでピッキングした音。
    MuteDown,
    /// パームミュートしてアップストロークでピッキングした音。
    MuteUp,
    /// ピッキングハーモニクス（ピンチ）。
    PinchHarmonic,
    /// 低い音から滑り上がって着く音。幅は CC26（[`SLIDE_WIDTH_CC`]）で選ぶ。
    SlideUp,
    /// 高い音から滑り下りて着く音。幅は CC26 で選ぶ。
    SlideDown,
    /// 半音下から持ち上げて着くチョーキング。
    BendHalf,
    /// 1 音下から持ち上げて着くチョーキング。
    BendWhole,
    /// 1 音半下から持ち上げて着くチョーキング。
    BendWholeHalf,
    /// ピックの縁で弦を擦る効果音（ピックスクレイプ）。音高は擦りの速さ（[`PICK_SCRATCH_PITCHES`]）。
    PickScratch,
}

impl Articulation {
    /// 全 variant。KS 番号から奏法を引く候補。
    pub const ALL: [Articulation; 13] = [
        Articulation::SusDown,
        Articulation::SusUp,
        Articulation::HammerOn,
        Articulation::PullOff,
        Articulation::MuteDown,
        Articulation::MuteUp,
        Articulation::PinchHarmonic,
        Articulation::SlideUp,
        Articulation::SlideDown,
        Articulation::BendHalf,
        Articulation::BendWhole,
        Articulation::BendWholeHalf,
        Articulation::PickScratch,
    ];

    /// この奏法を選ぶ KS の note number。
    pub fn keyswitch(self) -> u8 {
        match self {
            Articulation::SusDown => 17,
            Articulation::SusUp => 18,
            Articulation::HammerOn => 26,
            Articulation::PullOff => 25,
            Articulation::MuteDown => 20,
            Articulation::MuteUp => 21,
            Articulation::PinchHarmonic => 10,
            Articulation::SlideUp => 24,
            Articulation::SlideDown => 23,
            Articulation::BendHalf => 91,
            Articulation::BendWhole => 92,
            Articulation::BendWholeHalf => 93,
            Articulation::PickScratch => 8,
        }
    }

    /// METAL-GTX の KS 一覧（`METAL-GTX_KSMap.txt`）での名前。
    pub fn name(self) -> &'static str {
        match self {
            Articulation::SusDown => "Sus_Down",
            Articulation::SusUp => "Sus_Up",
            Articulation::HammerOn => "Hammer-On",
            Articulation::PullOff => "Pull-Off",
            Articulation::MuteDown => "Mute_Down",
            Articulation::MuteUp => "Mute_Up",
            Articulation::PinchHarmonic => "PH",
            Articulation::SlideUp => "Slide_Up",
            Articulation::SlideDown => "Slide_Down",
            Articulation::BendHalf => "Bending_HT",
            Articulation::BendWhole => "Bending_WH",
            Articulation::BendWholeHalf => "Bending_1HT",
            Articulation::PickScratch => "Pick_Scratch",
        }
    }

    /// KS の note number から奏法を引く。KS でない音高なら `None`。
    pub fn from_keyswitch(key: u8) -> Option<Self> {
        Articulation::ALL
            .into_iter()
            .find(|articulation| articulation.keyswitch() == key)
    }
}

/// 列ごとに ON/OFF するルール。次のルールはここへ variant を足す。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    /// 前の列から上行ならハンマリング、下行ならプリングオフ。
    HammerPull,
    /// ピッキングする音をパームミュートにする（ストロークの D/U は保つ）。
    PalmMute,
    /// ピッキングする音をピッキングハーモニクスにする。
    PinchHarmonic,
    /// 前の列から滑って着く（上行なら `Slide_Up`、下行なら `Slide_Down`）。
    Slide,
    /// 前の列より上の音へ、下から持ち上げて着く。
    Choke,
    /// 音を伸ばしている間、音程を揺らす（CC20、[`VIBRATO_DEPTH_CC`]）。他のルールと重ねられる。
    Vibrato,
    /// 列を、音の長さだけ擦るピックスクレイプにする（[`apply_pick_scratch`]）。
    PickScratch,
}

impl Rule {
    /// 「どう鳴らすか」を決めるルールか。1 音に KS は 1 つなので、同じ列ではこのうち 1 つだけ ON にできる。
    pub fn is_exclusive(self) -> bool {
        match self {
            Rule::HammerPull
            | Rule::PalmMute
            | Rule::PinchHarmonic
            | Rule::Slide
            | Rule::Choke
            | Rule::PickScratch => true,
            Rule::Vibrato => false,
        }
    }
}

/// 行全体で ON/OFF するルール。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowRule {
    /// [`auto_pick_columns`] の列だけピッキングし、残りを H/P にする。
    AutoHammerPull,
    /// イングヴェイ流のエコノミーピッキング（[`articulate`]）。
    EconomyPicking,
    /// 音ごとに時刻・velocity・ピッキングノイズの量をばらつかせる（汚し）。
    Humanize,
    /// 列ごとにリリース音の種類（CC24）と音量（CC25）をばらつかせる（汚し（リリース））。
    HumanizeRelease,
}

impl RowRule {
    /// 同時に ON にできない相手。自動ハンマリングとエコノミーピッキングはどちらも
    /// 各音をダウン/アップ/ハンマリング/プリングのどれにするかを決めるので、片方だけ効かせる。
    fn excluded(self) -> Option<RowRule> {
        match self {
            RowRule::AutoHammerPull => Some(RowRule::EconomyPicking),
            RowRule::EconomyPicking => Some(RowRule::AutoHammerPull),
            RowRule::Humanize | RowRule::HumanizeRelease => None,
        }
    }
}

/// 列番号 → その列で ON のルールと、行全体で ON のルール。
///
/// JSON では `{"columns":{"3":["hammer_pull"]},"rows":["economy_picking"]}`。
/// 排他な行ルール（[`RowRule::excluded`]）が両方 ON の JSON は、エコノミーピッキングだけ残して読む。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "RuleTableFields")]
pub struct RuleTable {
    #[serde(rename = "columns")]
    on: BTreeMap<usize, BTreeSet<Rule>>,
    rows: BTreeSet<RowRule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleTableFields {
    #[serde(default)]
    columns: BTreeMap<usize, BTreeSet<Rule>>,
    #[serde(default)]
    rows: BTreeSet<RowRule>,
}

impl From<RuleTableFields> for RuleTable {
    fn from(fields: RuleTableFields) -> Self {
        let mut rows = fields.rows;
        if rows.contains(&RowRule::EconomyPicking) {
            rows.remove(&RowRule::AutoHammerPull);
        }
        RuleTable {
            on: fields.columns,
            rows,
        }
    }
}

impl RuleTable {
    /// 1 行の JSON。ログから演奏を作り直すための綴り。
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("RuleTable is always serializable")
    }

    pub fn from_json(json: &str) -> Result<RuleTable, String> {
        serde_json::from_str(json).map_err(|err| format!("ルールの JSON を読めません: {err}"))
    }

    pub fn is_on(&self, column: usize, rule: Rule) -> bool {
        self.on
            .get(&column)
            .is_some_and(|rules| rules.contains(&rule))
    }

    /// ON/OFF を切り替える。[`Rule::is_exclusive`] なルールを ON にすると、同じ列の他の排他なルールは OFF になる。
    pub fn toggle(&mut self, column: usize, rule: Rule) {
        let rules = self.on.entry(column).or_default();
        if !rules.remove(&rule) {
            if rule.is_exclusive() {
                rules.retain(|other| !other.is_exclusive());
            }
            rules.insert(rule);
        }
        if rules.is_empty() {
            self.on.remove(&column);
        }
    }

    /// 列ごとのルールが 1 つも ON でないか。行全体のルールは見ない。
    pub fn is_empty(&self) -> bool {
        self.on.is_empty()
    }

    /// そのルールが ON の列の数。
    pub fn column_count_of(&self, rule: Rule) -> usize {
        self.on
            .values()
            .filter(|rules| rules.contains(&rule))
            .count()
    }

    pub fn is_row_on(&self, rule: RowRule) -> bool {
        self.rows.contains(&rule)
    }

    /// ON/OFF を切り替える。ON にすると、排他な相手（[`RowRule::excluded`]）は OFF になる。
    pub fn toggle_row(&mut self, rule: RowRule) {
        if !self.rows.remove(&rule) {
            if let Some(other) = rule.excluded() {
                self.rows.remove(&other);
            }
            self.rows.insert(rule);
        }
    }

    /// 列ごとのルールだけを消した表。MML を確定し直すと列の位置が意味を失うので使う。
    pub fn without_column_rules(&self) -> RuleTable {
        RuleTable {
            on: BTreeMap::new(),
            rows: self.rows.clone(),
        }
    }

    /// その列で H/P を効かせるか（列ごとの ON か、自動ハンマリングがレガートにする列）。
    /// `auto_picks` は [`auto_pick_columns`] の出力。
    pub fn hammer_pull_applies(&self, column: usize, auto_picks: &[bool]) -> bool {
        self.is_on(column, Rule::HammerPull)
            || (self.is_row_on(RowRule::AutoHammerPull) && !auto_picks[column])
    }
}

/// rawの列に、ルール表から作った KS と CC を足した演奏用の列を返す。
///
/// 元のイベントは時刻を変えずに残し、note on の velocity だけ [`articulate`] の値へ差し替える。
/// KS を元の列より前に積んでから stable sort するので、同時刻では KS の note on が
/// 演奏音の note on より前に来る（後だとその音に KS が効かない）。CC は並べ替えで同時刻の
/// note on より前に来る。
///
/// [`RowRule::Humanize`] が ON なら、演奏音の時刻と velocity を汚しの値で作り直し、KS と CC も
/// ずらした時刻から作る（音ごとの結果は固定の seed で決まる）。
///
/// [`RowRule::HumanizeRelease`] が ON なら、演奏音と同じ時刻の音から列ごとの CC24 / CC25 を足す
/// （他のイベントは ON/OFF で変わらない）。
pub fn convert(events: &[TimedMidiEvent], rules: &RuleTable) -> Vec<TimedMidiEvent> {
    let notes = notes_from_events(events);
    let articulated = articulate(&notes, rules);
    let release = rules.is_row_on(RowRule::HumanizeRelease);
    if rules.is_row_on(RowRule::Humanize) {
        let humanized = humanize::seeded(&notes, &articulated);
        let mut out = humanize::humanized_events(events, &notes, &articulated, &humanized, rules);
        if release {
            out.extend(release::seeded_release_events(&humanize::shifted_notes(
                &notes, &humanized,
            )));
        }
        cmrt_midi_filter::sort_for_playback(&mut out);
        return out;
    }
    let articulations: Vec<Articulation> = articulated.iter().map(|a| a.articulation).collect();
    let mut out = keyswitch_events(&notes, &articulations);
    out.extend(control_events(&notes, &articulations, rules));
    out.extend(events.iter().filter_map(|event| {
        let mut event = *event;
        if let Some(velocity) = picking::velocity_for(&event, &notes, &articulated) {
            event.message[2] = velocity;
        }
        scratch::sounding_event(&event, &notes, &articulated)
    }));
    if release {
        out.extend(release::seeded_release_events(&notes));
    }
    cmrt_midi_filter::sort_for_playback(&mut out);
    out
}
