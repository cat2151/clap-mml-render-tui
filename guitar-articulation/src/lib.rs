//! rawの MIDI イベント列を、METAL-GTX（sfz）のキースイッチ入りの列へ変換する。
//!
//! 変換は 1 ルール = 1 パスの純関数を順に積む（[`convert`] がその並び）。
//! KS 番号は METAL-GTX 固有なので、汎用の `cmrt-midi-filter` には置かない。
//! 変換を試す画面（[`GuitarArticulationScreen`] と [`ui`]）も同じ crate に置く。

mod accent;
mod arp;
mod articulation;
mod auto_pick;
mod column_map;
mod column_rule_anchor;
mod column_sound;
mod control;
mod glide;
mod hammer_pull;
mod history;
mod humanize;
mod instrument;
mod keyswitch;
mod notes;
mod params;
mod picking;
mod release;
mod report;
mod rule_table;
mod sample_midi;
mod scratch;
mod screen;
mod settings;
mod single_note;
mod strings;
pub mod ui;
mod unison_bend;
mod voicing;

#[cfg(test)]
mod test_effects;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

pub use accent::AccentPattern;
pub use arp::{
    arpeggiate, performance_events, ArpSettings, ARP_CYCLES, ARP_OCTAVES, ARP_PATTERNS, ARP_TURN,
};
pub use articulation::Articulation;
pub use auto_pick::{auto_pick_columns, AutoPick, RUN_MIN_NOTES, RUN_PICK_EVERY};
pub use cmrt_midi_filter::TimedMidiEvent;
pub use column_rule_anchor::ColumnRuleAnchor;
pub use column_sound::{
    fold_pitch, CHROMATIC_RUN_PITCHES, EFFECT_HARD_STOP_PITCH, EFFECT_HELLO_PITCH,
    EFFECT_RESONANCE_PITCH, EFFECT_SLIDE_NOISE_PITCH, SLIDE_FX_DOWN_PITCHES, SLIDE_FX_UP_PITCHES,
};
pub use control::{
    control_events, AUTO_SLIDE_OUT_PITCHES, AUTO_SLIDE_OUT_VALUE, LONG_EXTRA_CC,
    POSITION_RELEASE_PITCHES, POSITION_RELEASE_VALUE, POWER_CHORD_CC, POWER_CHORD_PITCHES,
    SLIDE_IN_WIDTH_CC, SLIDE_IN_WIDTH_CC_DEFAULT, SLIDE_WIDTH_CC, SLIDE_WIDTH_CC_DEFAULT,
    VIBRATO_DEPTH, VIBRATO_DEPTH_CC,
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
pub use params::{param_of, Param, PARAMS, PARAM_STEP};
pub use picking::{articulate, Articulated, UNACCENTED_PICK_VELOCITY_PERCENT};
pub use report::{compare, report};
pub use rule_table::RuleTable;
pub use sample_midi::{
    is_keyswitch_pitch, keyswitch_name, SampleMidi, SampleMidiGroup, SAMPLE_MIDI_DIR,
};
pub use scratch::{pick_scratch_pitch, PICK_SCRATCH_PITCHES};
pub use screen::{GuitarArticulationAction, GuitarArticulationScreen, Take, DEFAULT_MML};
pub use settings::{load_settings, save_settings, GuitarArticulationSettings};
pub use single_note::column_events;
pub use strings::{picks_string, strings_by_column, REACH_SEMITONES};
pub use unison_bend::{
    UNISON_BEND_FALL_SECONDS, UNISON_BEND_RISE_SECONDS, UNISON_BEND_STEP_SECONDS,
};
pub use voicing::{
    apply_voicing_rules, slide_in_pitches, slide_in_width, BRUSH_PITCHES, FRET_MUTE_PITCHES,
    GLIDE_IN_PITCHES, MUTE_PITCHES, NATURAL_HARMONICS_PITCHES, PINCH_HARMONIC_PITCHES,
    SLIDE_OUT_PITCHES, TRILL_PITCHES, UNISON_BEND_PITCHES,
};

/// 画面に入ったときに読む音色（sforzando の `patches_dirs` からの相対）。KS 番号が METAL-GTX に固有なので固定する。
pub const PATCH: &str = "sfz/UI_METAL-GTX/Programs/02-METAL-GTX Lite.sfz";

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
    /// 幅は最大 7 半音で、それより広い音程は 7 半音手前まで跳んでから滑る。
    Slide,
    /// 前の列より上の音へ、下から持ち上げて着く。
    Choke,
    /// 音を伸ばしている間、音程を揺らす（CC20、[`VIBRATO_DEPTH_CC`]）。他のルールと重ねられる。
    Vibrato,
    /// 列を、音の長さだけ擦るピックスクレイプにする（[`pick_scratch_pitch`]）。
    ///
    /// ここから下の列ルールは、列のいちばん低い音 1 つだけを鳴らし（擦る音や効果音は和音にならない）、
    /// 列の他の奏法を上書きする。
    PickScratch,
    /// ピッキングする音をナチュラルハーモニクスにする。
    NaturalHarmonics,
    /// ピッキングする音をブラッシングにする（ストロークの D/U は保つ）。
    Brushing,
    /// ピッキングする音をフレットミュートにする（ストロークの D/U は保つ）。
    FretMute,
    /// ピッキングする音を、伸ばした後に滑り下りて離す音にする。
    SlideOut,
    /// ピッキングする音と H/P の音を、前の音から滑らせる擬似レガートにする。
    PseudoLegato,
    /// ピッキングする音と H/P の音を、前の音から滑るポルタメントにする。
    Portamento,
    /// ピッキングする音を、前の列からの音程の幅で下から滑り込むスライドインにする（[`slide_in_width`]）。
    SlideIn,
    /// ピッキングする音を、半音上とのトリルにする。
    TrillHalf,
    /// ピッキングする音を、全音上とのトリルにする。
    TrillWhole,
    /// ピッキングする音を、短 3 度上とのトリルにする。
    TrillMinorThird,
    /// ピッキングする音を、長 3 度上とのトリルにする。
    TrillMajorThird,
    /// ピッキングする音を、ユニゾンチョーキング（自動）にする。
    UnisonBendAuto,
    /// ピッキングする音を、pitch bend で持ち上げるユニゾンチョーキング（手動）にする
    /// （[`UNISON_BEND_RISE_SECONDS`]）。列の音が鳴っている間に他の列の音が重なる列では効かない。
    UnisonBendManual,
    /// 列を、クロマチックランのフレーズ 1 つにする（[`CHROMATIC_RUN_PITCHES`]）。
    ChromaticRun,
    /// 列を、滑り下りる効果音にする。
    SlideFxDown,
    /// 列を、滑り上がる効果音にする。
    SlideFxUp,
    /// 列を、上下に滑る効果音にする。
    SlideFxWow,
    /// 列を、効果音 `Hello!`（[`EFFECT_HELLO_PITCH`]）にする。
    EffectHello,
    /// 列を、弦の共鳴の効果音（[`EFFECT_RESONANCE_PITCH`]）にする。
    EffectResonance,
    /// 列を、スライドノイズの効果音（[`EFFECT_SLIDE_NOISE_PITCH`]）にする。
    EffectSlideNoise,
    /// 列を、ハードストップの効果音（[`EFFECT_HARD_STOP_PITCH`]）にする。
    EffectHardStop,
    /// Sus_Down / Mute_Down の列を、長め・強めの sample（Sus_LT / Sus_EX / Mute_EX）にする
    /// （CC23、[`LONG_EXTRA_CC`]）。他のルールと重ねられる。
    LongExtra,
    /// Sus_Down / Sus_Up の列に、5 度上の音を重ねてパワーコードにする（CC32、[`POWER_CHORD_CC`]）。
    /// 音高は [`POWER_CHORD_PITCHES`] の中だけ。他のルールと重ねられる。
    PowerChord,
    /// 列の音を離したときに、手のポジション移動の音を鳴らす（CC24 = [`POSITION_RELEASE_VALUE`]）。
    /// 他のルールと重ねられ、汚し（リリース）より優先する。
    PositionRelease,
    /// 列の音を離したときに、滑り下りる音を鳴らす（自動スライドアウト、CC24 = [`AUTO_SLIDE_OUT_VALUE`]）。
    /// 列の音の奏法は変えない。[`Rule::SlideOut`] は列の音そのものを滑り下りる音に置き換える。
    /// 他のルールと重ねられ、汚し（リリース）より優先する。[`Rule::PositionRelease`] とは同じ列で 1 つだけ。
    AutoSlideOut,
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
            | Rule::PickScratch
            | Rule::NaturalHarmonics
            | Rule::Brushing
            | Rule::FretMute
            | Rule::SlideOut
            | Rule::PseudoLegato
            | Rule::Portamento
            | Rule::SlideIn
            | Rule::TrillHalf
            | Rule::TrillWhole
            | Rule::TrillMinorThird
            | Rule::TrillMajorThird
            | Rule::UnisonBendAuto
            | Rule::UnisonBendManual
            | Rule::ChromaticRun
            | Rule::SlideFxDown
            | Rule::SlideFxUp
            | Rule::SlideFxWow
            | Rule::EffectHello
            | Rule::EffectResonance
            | Rule::EffectSlideNoise
            | Rule::EffectHardStop => true,
            Rule::Vibrato
            | Rule::LongExtra
            | Rule::PowerChord
            | Rule::PositionRelease
            | Rule::AutoSlideOut => false,
        }
    }

    /// 離したときのリリース層を CC24 で選ぶルールか。1 列で効く CC24 は 1 つなので、同じ列ではこのうち 1 つだけ ON にできる。
    pub fn selects_release_shape(self) -> bool {
        matches!(self, Rule::PositionRelease | Rule::AutoSlideOut)
    }

    /// 同じ列で一緒に ON にできないか（[`Rule::is_exclusive`] どうし、[`Rule::selects_release_shape`] どうし）。
    fn conflicts_with(self, other: Rule) -> bool {
        (self.is_exclusive() && other.is_exclusive())
            || (self.selects_release_shape() && other.selects_release_shape())
    }
}

/// 行全体で ON/OFF するルール。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowRule {
    /// [`RuleTable::auto_pick`] の列だけピッキングし、残りを H/P にする。
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
/// （他のイベントは ON/OFF で変わらない）。[`Rule::PositionRelease`] の列の CC24 は乱数の代わりに
/// 列のルールが送る値（[`POSITION_RELEASE_VALUE`] / [`AUTO_SLIDE_OUT_VALUE`]）。
///
/// 最後に、演奏の頭で列 CC の既定値を足す（[`control::add_column_cc_defaults`]）。
pub fn convert(events: &[TimedMidiEvent], rules: &RuleTable) -> Vec<TimedMidiEvent> {
    let notes = notes_from_events(events);
    let articulated = articulate(&notes, rules);
    let release = rules.is_row_on(RowRule::HumanizeRelease);
    let articulations: Vec<Articulation> = articulated.iter().map(|a| a.articulation).collect();
    if rules.is_row_on(RowRule::Humanize) {
        let humanized = humanize::seeded(&notes, &articulated);
        let mut out = humanize::humanized_events(events, &notes, &articulated, &humanized, rules);
        let shifted = humanize::shifted_notes(&notes, &humanized);
        if release {
            let positions = control::release_shape_columns(&shifted, &articulations, rules);
            out.extend(release::seeded_release_events(&shifted, &positions));
        }
        control::add_column_cc_defaults(&mut out, &shifted);
        cmrt_midi_filter::sort_for_playback(&mut out);
        return out;
    }
    let mut out = keyswitch_events(&notes, &articulations);
    out.extend(control_events(&notes, &articulations, rules));
    out.extend(events.iter().filter_map(|event| {
        let mut event = *event;
        if let Some(velocity) = picking::velocity_for(&event, &notes, &articulated) {
            event.message[2] = velocity;
        }
        column_sound::sounding_event(&event, &notes, &articulated)
    }));
    if release {
        let positions = control::release_shape_columns(&notes, &articulations, rules);
        out.extend(release::seeded_release_events(&notes, &positions));
    }
    control::add_column_cc_defaults(&mut out, &notes);
    cmrt_midi_filter::sort_for_playback(&mut out);
    out
}
