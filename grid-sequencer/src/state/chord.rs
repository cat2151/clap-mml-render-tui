//! chord mode の再生状態。
//!
//! 抽選したコード進行を grid 1周（16ステップ = 全音符1個）ごとに1コードずつ進め、
//! [`CHORD_ROW`] の instance で和音として鳴らす。ほかの行はリズムをそのままに、
//! note number だけを現在のコードの構成音へ寄せる。
//!
//! コード進行そのものの抽選（カタログと rng）は画面側の仕事。ここは「与えられた
//! 進行をどう鳴らすか」だけを持つ。

use std::time::Instant;

use cmrt_chord::ChordVoicing;

use super::{GridInstance, GridLaneMode, GridScheduledMessage, GridState, LaneAddress};

/// 和音を鳴らす行。UI の行1 = realtime play server の instance 0。
pub const CHORD_ROW: usize = 0;

/// bass を鳴らす行。UI の行2。和音とは別の patch で、lane の pattern をそのまま使う。
/// [`super::GridLaneMode::BassOctave2`] の行で、lane 0 = bass 音・lane 1 = 1オクターブ上
/// （[`super::bass_line`]）。
pub const BASS_ROW: usize = 1;

/// アルペジオを書く行。UI の行3。[`super::GridLaneMode::ChordVoices4`] の既定行で、
/// 4 voice を独立した pattern で鳴らす（[`super::arpeggio`]）。
pub const ARPEGGIO_ROW: usize = 2;

/// 抽選済みのコード進行と、その再生位置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordPlayback {
    key: &'static str,
    degrees: String,
    /// コード1つぶんの note number 群を進行の順に並べたもの。空にはしない。
    chords: Vec<Vec<u8>>,
    /// コード1つにつき1音の bass。`chords` と同じ長さ。
    /// auto voicing を通していない経路では全て `None` になり、bass 行は鳴らない。
    basses: Vec<Option<u8>>,
    index: usize,
}

impl ChordPlayback {
    /// 空の進行は再生できないので `None` を返す。
    ///
    /// bass は付かない。auto voicing を通した実経路では [`Self::from_voicings`] を使う。
    pub fn new(key: &'static str, degrees: String, chords: Vec<Vec<u8>>) -> Option<Self> {
        if chords.is_empty() {
            return None;
        }
        let basses = vec![None; chords.len()];
        Some(Self {
            key,
            degrees,
            chords,
            basses,
            index: 0,
        })
    }

    /// auto voicing 済みの進行から作る。空の進行は再生できないので `None` を返す。
    pub fn from_voicings(
        key: &'static str,
        degrees: String,
        voicings: Vec<ChordVoicing>,
    ) -> Option<Self> {
        if voicings.is_empty() {
            return None;
        }
        let mut chords = Vec::with_capacity(voicings.len());
        let mut basses = Vec::with_capacity(voicings.len());
        for voicing in voicings {
            chords.push(voicing.notes);
            basses.push(voicing.bass);
        }
        Some(Self {
            key,
            degrees,
            chords,
            basses,
            index: 0,
        })
    }

    pub fn key(&self) -> &str {
        self.key
    }

    pub fn degrees(&self) -> &str {
        &self.degrees
    }

    /// 進行に含まれるコードの数。1以上であることが構築時に保証されている。
    pub fn chord_count(&self) -> usize {
        self.chords.len()
    }

    /// 進行内の現在位置（0 始まり）。
    pub fn index(&self) -> usize {
        self.index
    }

    /// いま鳴らすべき和音の note number 群。
    pub fn current(&self) -> &[u8] {
        &self.chords[self.index]
    }

    /// いま鳴らすべき bass 音。auto voicing を通していない進行では `None`。
    pub fn current_bass(&self) -> Option<u8> {
        self.basses.get(self.index).copied().flatten()
    }

    /// 進行内の `index` 番目の voicing。次の進行を接続するための seed に使う。
    pub fn voicing_at(&self, index: usize) -> Option<ChordVoicing> {
        Some(ChordVoicing {
            bass: *self.basses.get(index)?,
            notes: self.chords.get(index)?.clone(),
        })
    }

    /// いま鳴らしている voicing。
    pub fn current_voicing(&self) -> Option<ChordVoicing> {
        self.voicing_at(self.index)
    }

    /// 進行の最後の voicing。次サイクルへ差し替える直前に鳴っているコード。
    pub fn last_voicing(&self) -> Option<ChordVoicing> {
        self.voicing_at(self.chords.len().checked_sub(1)?)
    }

    pub fn max_voice_count(&self) -> usize {
        self.chords.iter().map(Vec::len).max().unwrap_or(0)
    }

    /// 進行内のどのコードでも鳴る声部数。アルペジオの声部数はこれで決める。
    ///
    /// [`Self::max_voice_count`] で組むと、構成音の少ないコードの周だけ上の声部が
    /// 無音になり、リズムが欠ける。
    pub fn min_voice_count(&self) -> usize {
        self.chords.iter().map(Vec::len).min().unwrap_or(0)
    }

    /// 同じ進行を頭から鳴らし直す複製。
    ///
    /// 進行を引き直さない周（CHORD を OFF にした周）でも、次サイクルへは
    /// 「先頭から1周ぶん」を預ける必要がある。最終小節を指したまま預けると、
    /// 差し替えた次の周が最後のコードから始まってしまう。
    pub fn restarted(&self) -> Self {
        Self {
            index: 0,
            ..self.clone()
        }
    }

    /// 次のコードへ進める。進行を1周して先頭へ戻ったときだけ true。
    fn advance(&mut self) -> bool {
        self.index = (self.index + 1) % self.chords.len();
        self.index == 0
    }

    /// いま鳴らしているのが進行の最後のコードか（= この小節が最終小節）。
    /// 待機 bank への先読みロードを始める合図に使う。
    pub(super) fn is_last(&self) -> bool {
        self.index + 1 == self.chords.len()
    }

    /// 和音に含まれるピッチクラス（0〜11）の集合。
    pub(super) fn pitch_classes(&self) -> [bool; 12] {
        let mut classes = [false; 12];
        for note in self.current() {
            classes[usize::from(note % 12)] = true;
        }
        classes
    }
}

impl GridState {
    /// chord mode を切り替える。`None` で解除。
    ///
    /// 鳴っている音を止める note off を返す。呼び出し側は必ず送ること
    /// （音色ロードを伴わない切替では消音が他に走らないため）。
    pub fn set_chord(
        &mut self,
        chord: Option<ChordPlayback>,
        now: Instant,
    ) -> Vec<GridScheduledMessage> {
        self.chord = chord;
        self.discard_pending_cycle();
        self.refresh_lane_display_patterns();
        self.take_silence_messages(now)
    }

    /// grid を1周したときに次のコードへ進める。
    ///
    /// 進行を1周し終えたら、準備済みの次サイクルへ差し替える（[`super::cycle`]）。
    /// 音色を引き直す周は待機 bank へ移り、据え置く周は現在 bank のまま進行だけを
    /// 取り込む。
    /// 差し替えは `attack_current_step()` の直前に起きるので、その小節の頭から
    /// 新しい進行の1コード目がそのまま鳴る。
    ///
    /// シングルバッファリング（[`crate::single_buffer`]）では裏読みをしていないので
    /// 差し替えず、「鳴らしきった」合図だけを立てて `poll_steps` にクロックを畳ませる。
    ///
    /// 進行を1周して先頭のコードへ戻ったときだけ true を返す。テンポの引き直しは
    /// 小節ではなくこの進行1周を単位にするので（[`super::tempo`]）、その境目を
    /// 知っているここが報告する。鳴らしきって止まる周は、次の音が無いので false。
    pub(super) fn advance_chord(&mut self) -> bool {
        let Some(chord) = self.chord.as_mut() else {
            return false;
        };
        let wrapped = chord.advance();
        if wrapped {
            if self.stop_at_cycle_end {
                self.cycle_wrapped = true;
                return false;
            }
            self.commit_pending_cycle();
        }
        // 最終小節へ入った。画面側はここから次の抽選と先読みロードを始める。
        if self.chord.as_ref().is_some_and(ChordPlayback::is_last) {
            self.preload_due = true;
        }
        wrapped
    }

    /// 保存値ではなく現在の mode / chord から、その lane が次に鳴らす音を導出する。
    pub fn resolved_note(&self, address: LaneAddress) -> Option<u8> {
        resolved_note_from(&self.instances, self.chord.as_ref(), address)
    }
}

pub(super) fn resolved_note_from(
    instances: &[GridInstance],
    chord: Option<&ChordPlayback>,
    address: LaneAddress,
) -> Option<u8> {
    let instance = instances.get(address.instance)?;
    let lane = instance.lanes.get(address.lane)?;
    let Some(chord) = chord else {
        return (address.lane == 0).then_some(lane.base_note);
    };
    if address.instance == CHORD_ROW {
        // chord instance は個別laneではなく summary owner が全構成音を鳴らす。
        return None;
    }
    if address.instance == BASS_ROW {
        // bass 行は lane の pattern をそのまま使い、音高だけをコードの bass 音へ固定する。
        // lane 0 = bass 音、lane 1 = その1オクターブ上。保存値の base_note は見ない。
        return bass_octave_note(chord.current_bass(), address.lane);
    }
    match instance.lane_mode {
        GridLaneMode::Single => Some(snap_to_chord(lane.base_note, &chord.pitch_classes())),
        // BassOctave2 は BASS_ROW 専用で、その分岐は上で返している。ここへ来るのは
        // lane_mode だけが残った壊れた状態なので、音を重ねないよう lane 0 に絞る。
        GridLaneMode::BassOctave2 => {
            (address.lane == 0).then(|| snap_to_chord(lane.base_note, &chord.pitch_classes()))
        }
        GridLaneMode::ChordVoices4 => {
            rotated_chord_voice(chord.current(), address.lane, instance.voicing_rotation)
        }
        // drum 行はコードへ寄せない。寄せると kick の音高が小節ごとに動いてしまう。
        GridLaneMode::Drum => (address.lane == 0).then_some(lane.base_note),
    }
}

/// bass 行の lane を音高へ解決する。lane 0 は bass 音そのまま、lane 1 は1オクターブ上。
///
/// [`super::BASS_OCTAVE_LANES`] を超える lane（旧セッションが残した余りの lane）と、
/// MIDI note number の上限を越えるオクターブ上は鳴らさない。
fn bass_octave_note(bass: Option<u8>, lane: usize) -> Option<u8> {
    let bass = bass?;
    match lane {
        0 => Some(bass),
        1 => bass.checked_add(12).filter(|note| *note <= 127),
        _ => None,
    }
}

/// signedな`rotation`ぶん構成音を累積してずらす。正は上へ、負は下へ転回する。
///
/// C-E-Gを例にすると、`1`はE-G-C5、`-1`はG3-C-E、`-3`はC3-E3-G3になる。
pub(super) fn rotated_chord_voice(notes: &[u8], lane: usize, rotation: i8) -> Option<u8> {
    let notes = &notes[..notes.len().min(super::CHORD_VOICE_LANES)];
    let rendered_voice_count = if notes.len() == 3 {
        super::CHORD_VOICE_LANES
    } else {
        notes.len()
    };
    if lane >= rendered_voice_count || notes.is_empty() {
        return None;
    }
    let note_count = i16::try_from(notes.len()).ok()?;
    let rotation = i16::from(rotation);
    let mut previous = None;
    for voice in 0..=lane {
        let sequence = rotation + i16::try_from(voice).ok()?;
        let note_index = usize::try_from(sequence.rem_euclid(note_count)).ok()?;
        let mut note = i16::from(notes[note_index]) + 12 * sequence.div_euclid(note_count);
        while previous.is_some_and(|previous| note <= previous) {
            note += 12;
        }
        if !(0..=127).contains(&note) {
            return None;
        }
        previous = Some(note);
    }
    previous.map(|note| note as u8)
}

/// `base` に最も近い、ピッチクラスが `classes` に含まれる note number を返す。
///
/// 同距離なら低いほうを選ぶ（上へ寄って音域が上ずるのを防ぐ）。
pub(super) fn snap_to_chord(base: u8, classes: &[bool; 12]) -> u8 {
    if !classes.iter().any(|on| *on) {
        return base;
    }
    // 3和音以上ならピッチクラスの間隔は最大でも半音6個ぶんなので、距離6まで見れば必ず当たる。
    for distance in 0..=6 {
        if let Some(down) = base.checked_sub(distance) {
            if classes[usize::from(down % 12)] {
                return down;
            }
        }
        if let Some(up) = base.checked_add(distance) {
            if up <= 127 && classes[usize::from(up % 12)] {
                return up;
            }
        }
    }
    base
}

#[cfg(test)]
mod tests;
