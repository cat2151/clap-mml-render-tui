//! ベースライン。root と octave 上の2声だけを、一定の刻みで敷き詰める。
//!
//! アルペジオ（[`super::arp`]）と違って rng を引かない。同じ [`BassPattern`] からは
//! 必ず同じ譜面が出るので、wheel で送った種別と鳴る内容が1対1で対応する。
//!
//! 音高は持たない。声部を実際の note number へ解決するのは呼び出し側の責務で、
//! [`BASS_ROOT_VOICE`] がコードの bass 音、[`BASS_OCTAVE_VOICE`] がその1オクターブ上。

mod pattern;

pub use pattern::BassPattern;

/// コードの bass 音をそのまま鳴らす声部。
pub const BASS_ROOT_VOICE: usize = 0;
/// bass 音の1オクターブ上を鳴らす声部。
pub const BASS_OCTAVE_VOICE: usize = 1;

/// 1拍の step 数（16分音符 4つ）。[`BassPattern::beat_rhythm`] はこの長さを刻む。
const BEAT_STEPS: usize = 4;

/// octave bass が octave を切り替える単位（八分音符 = 2 step）。
///
/// 拍を細かく刻むパターンでも切替はこの単位のまま。`#-##` なら
/// root(八分) → octave(16分) → octave(16分) になる。
const OCTAVE_SWITCH_STEPS: usize = 2;

/// ベースライン1音。`voice` は [`BASS_ROOT_VOICE`] か [`BASS_OCTAVE_VOICE`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BassNote {
    pub step: usize,
    pub voice: usize,
    pub duration_steps: usize,
}

/// `pattern` を `steps` step ぶん展開する。`steps` が 0 なら空。
///
/// 1拍ぶんのリズムを先頭から繰り返し当てる。刻みで割り切れないぶんは、最後の音を
/// 小節末で切り詰める。
pub fn generate_bass_line(pattern: BassPattern, steps: usize) -> Vec<BassNote> {
    if steps == 0 {
        return Vec::new();
    }
    let Some(rhythm) = pattern.beat_rhythm() else {
        // 全音符は root 1音を小節いっぱい伸ばすだけ。
        return vec![BassNote {
            step: 0,
            voice: BASS_ROOT_VOICE,
            duration_steps: steps,
        }];
    };
    debug_assert_eq!(
        rhythm.iter().sum::<usize>(),
        BEAT_STEPS,
        "beat rhythm must fill exactly one beat"
    );
    let mut notes = Vec::new();
    let mut step = 0;
    for note_steps in rhythm.iter().copied().cycle() {
        if step >= steps {
            break;
        }
        notes.push(BassNote {
            step,
            voice: if pattern.uses_octave() {
                octave_voice(step)
            } else {
                BASS_ROOT_VOICE
            },
            duration_steps: note_steps.min(steps - step),
        });
        step += note_steps;
    }
    notes
}

/// 八分音符ごとに root と octave 上を往復する。
fn octave_voice(step: usize) -> usize {
    if (step / OCTAVE_SWITCH_STEPS).is_multiple_of(2) {
        BASS_ROOT_VOICE
    } else {
        BASS_OCTAVE_VOICE
    }
}

#[cfg(test)]
mod tests;
