use std::time::Instant;

use rand::RngExt;

use cmrt_arpeggiator::{generate_arpeggio, generate_bass_line, ArpPattern, BassPattern};
use cmrt_rhythm::{DrumPattern, DrumRole};
use cmrt_tui_core::random::random_index;

use crate::CycleRandom;

use super::{
    arpeggio::write_arpeggio, bass_line::write_bass_line, drum, ChordPlayback, GridInstance,
    GridScheduledMessage, GridState, NotePattern, PatternCombination, ARPEGGIO_ROW, BASS_ROW,
    GRID_STEPS, SWING_MAX, SWING_MIN,
};

/// ランダム生成に使う note number の範囲（C2〜C6）。
const RANDOM_NOTE_MIN: u8 = 36;
const RANDOM_NOTE_MAX: u8 = 84;
/// Rest位置でnote eventを開始する確率。密になりすぎず、無音にもなりにくい値。
const CELL_ON_RATIO: f64 = 0.25;
/// 1stepを多めにしつつ、2/4step noteも生成する重み付き候補。
const NOTE_LENGTHS: [usize; 6] = [1, 1, 1, 2, 2, 4];
/// アルペジオを組める最小の声部数。1声では音型にならない。
const MIN_ARP_VOICES: usize = 2;

/// 抽選で引いた「型」。Phrase pane のカーソルと NOTE grid のタイトルに出す。
///
/// 引かなかった種別は `None`。drum はroleごとに結果を持つ。最後の1つだけに潰すと、
/// loopごとに全roleを抽選できていても右paneでは1roleしか確認できないため。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawnPhrases {
    pub arp: Option<ArpPattern>,
    pub bass: Option<BassPattern>,
    drums: [Option<DrumPattern>; DrumRole::ALL.len()],
    last_drum: Option<DrumPattern>,
}

impl DrawnPhrases {
    pub(crate) fn with_arp(pattern: ArpPattern) -> Self {
        Self {
            arp: Some(pattern),
            ..Self::default()
        }
    }

    pub(crate) fn with_bass(pattern: BassPattern) -> Self {
        Self {
            bass: Some(pattern),
            ..Self::default()
        }
    }

    pub(crate) fn record_drum(&mut self, pattern: DrumPattern) {
        let index = DrumRole::ALL
            .iter()
            .position(|role| *role == pattern.role())
            .expect("DrumRole::ALL contains every role");
        self.drums[index] = Some(pattern);
        self.last_drum = Some(pattern);
    }

    pub(crate) fn drums(self) -> impl Iterator<Item = DrumPattern> {
        self.drums.into_iter().flatten()
    }

    pub(crate) fn drum_for(self, role: DrumRole) -> Option<DrumPattern> {
        let index = DrumRole::ALL
            .iter()
            .position(|candidate| *candidate == role)?;
        self.drums[index]
    }

    pub(crate) fn last_drum(self) -> Option<DrumPattern> {
        self.last_drum
    }

    pub(crate) fn merged(mut self, update: Self) -> Self {
        if update.arp.is_some() {
            self.arp = update.arp;
        }
        if update.bass.is_some() {
            self.bass = update.bass;
        }
        for (current, next) in self.drums.iter_mut().zip(update.drums) {
            if next.is_some() {
                *current = next;
            }
        }
        if update.last_drum.is_some() {
            self.last_drum = update.last_drum;
        }
        self
    }

    pub(crate) fn clear_drums(&mut self) {
        self.drums.fill(None);
        self.last_drum = None;
    }
}

/// instance群を丸ごと引き直す。`patches` が空なら patch だけ据え置く。
///
/// `GridState` の外へ出してあるのは、chord mode が「鳴っている grid を触らずに
/// 次サイクルを抽選する」ために、複製したinstance群へ同じ抽選をかけるため。
///
/// 何を引き直すかは `policy`（[`CycleRandom`]）で決まる。項目が重ならないよう、
/// **1行はちょうど1項目に属する**:
///
/// - drum 行 … DRUM（リズム型）
/// - chord mode 中の bass 行・アルペジオ行 … ARP（音型）
/// - それ以外の行 … NOTE（生の譜面と音高）
/// - 全行の patch … PATCH
///
/// `chord` は差し替え先の進行。`None`（chord mode OFF）なら bass 行もアルペジオ行も
/// 役割を持たないので、普通の行として NOTE で引かれる。
pub fn randomize_instance_slice(
    instances: &mut [GridInstance],
    patches: &[(String, String)],
    policy: CycleRandom,
    chord: Option<&ChordPlayback>,
    patterns: Option<PatternCombination>,
) -> DrawnPhrases {
    let mut rng = rand::rng();
    let mut drawn = DrawnPhrases::default();
    for (index, instance) in instances.iter_mut().enumerate() {
        if policy.patch {
            if let Some(patch_index) = random_index(patches.len()) {
                instance.patch = Some(patches[patch_index].0.clone());
            }
        }
        // swing は譜面の役割分けと直交する。drum の hi-hat こそ跳ねてほしい行なので、
        // 下の `continue` より手前で全行へ配る。実際に跳ねるかは譜面が決める
        // （[`super::swing`]）。
        if policy.swing {
            instance.swing = rng.random_range(SWING_MIN..=SWING_MAX);
        }
        // drum 行は音高も譜面も専用（[`super::drum`]）。音高を引き直すと打楽器の音色
        // そのものが変わり、汎用の譜面を当てるとリズムでなくなる。
        if let Some(role) = instance.drum {
            if policy.drum {
                let pattern = patterns
                    .and_then(|combination| combination.drum_pattern(role))
                    .expect("DRUM random requires a drum pattern combination");
                drum::write_drum_pattern(instance, pattern, &mut rng);
                drawn.record_drum(pattern);
            }
            continue;
        }
        // chord mode 中の bass 行・アルペジオ行は音高がコードから導出されるので、
        // 譜面は「型」から作る。生のランダム譜面を当てると、コードに乗らない
        // ばらばらのリズムになる。
        if let Some(chord) = chord.filter(|_| index == BASS_ROW || index == ARPEGGIO_ROW) {
            if policy.arp {
                randomize_phrase_row(instance, index, chord, patterns, &mut rng, &mut drawn);
            }
            continue;
        }
        if policy.note {
            for lane in &mut instance.lanes {
                lane.base_note = rng.random_range(RANDOM_NOTE_MIN..=RANDOM_NOTE_MAX);
                lane.pattern = random_pattern(&mut rng);
            }
        }
    }
    drawn
}

/// bass 行・アルペジオ行の譜面を、型を引いて書き直す。
fn randomize_phrase_row(
    instance: &mut GridInstance,
    index: usize,
    chord: &ChordPlayback,
    patterns: Option<PatternCombination>,
    rng: &mut impl RngExt,
    drawn: &mut DrawnPhrases,
) {
    if index == BASS_ROW {
        let pattern = patterns
            .and_then(PatternCombination::bass_pattern)
            .expect("ARP random requires a bass pattern combination");
        write_bass_line(instance, &generate_bass_line(pattern, GRID_STEPS));
        drawn.bass = Some(pattern);
        return;
    }
    // 進行内のどのコードでも鳴る声部数で組む。多いほうへ寄せると、構成音の少ない
    // コードの周だけ上の声部が無音になってリズムが欠ける。
    let voices = chord.min_voice_count().min(instance.lanes.len());
    if voices < MIN_ARP_VOICES {
        return;
    }
    let pattern = patterns
        .and_then(PatternCombination::arp_pattern)
        .expect("ARP random requires an arpeggio pattern combination");
    write_arpeggio(
        instance,
        &generate_arpeggio(pattern, voices, GRID_STEPS, rng),
    );
    drawn.arp = Some(pattern);
}

fn random_pattern(rng: &mut impl RngExt) -> NotePattern {
    let mut pattern = NotePattern::default();
    let mut step = 0;
    while step < GRID_STEPS {
        if !rng.random_bool(CELL_ON_RATIO) {
            step += 1;
            continue;
        }
        let length = NOTE_LENGTHS[rng.random_range(0..NOTE_LENGTHS.len())];
        let end = (step + length - 1).min(GRID_STEPS - 1);
        let _ = pattern.draw_span(step, end);
        step = end + 1;
    }
    pattern
}

impl GridState {
    /// 現在ONで、実際に画面へ存在するpattern対象のbagから1組引く。
    pub(crate) fn draw_pattern_combination(
        &mut self,
        policy: CycleRandom,
        chord_mode: bool,
    ) -> Option<PatternCombination> {
        let include_drums = policy.drum && self.instances.iter().any(|item| item.drum.is_some());
        let include_bass = policy.arp && chord_mode && self.instances.get(BASS_ROW).is_some();
        let include_arpeggio =
            policy.arp && chord_mode && self.instances.get(ARPEGGIO_ROW).is_some();
        self.pattern_bags
            .draw(include_drums, include_bass, include_arpeggio)
    }

    /// patch 一覧の非同期読み込み後、まだ未設定のinstanceだけへ音色を割り当てる。
    pub fn fill_missing_patches(&mut self, patches: &[(String, String)]) -> usize {
        let mut assigned = 0;
        for instance in &mut self.instances {
            if instance.patch.is_none() {
                if let Some(index) = random_index(patches.len()) {
                    instance.patch = Some(patches[index].0.clone());
                    assigned += 1;
                }
            }
        }
        assigned
    }

    /// 直近の引き直しで当てた型。表示のためだけに持つ（[`DrawnPhrases`]）。
    pub fn drawn_phrases(&self) -> DrawnPhrases {
        self.drawn
    }

    /// 全instanceのpatchと、全laneのnote number / note event patternを引き直す。
    ///
    /// 引き直す前に鳴っていた音の note off を返す（鳴りっぱなしを防ぐ）。
    /// 先読みで送信済みの note on より後ろへ置くため、note off には猶予が乗る。
    /// `patches` が空（読み込み中・エラー）のときは patch だけ据え置く。
    pub fn randomize_all(
        &mut self,
        now: Instant,
        patches: &[(String, String)],
    ) -> Vec<GridScheduledMessage> {
        self.randomize_instances(patches, CycleRandom::ALL);
        self.take_silence_messages(now)
    }

    /// 全instanceのpatchだけを引き直す。譜面（note / pattern）は据え置く。
    ///
    /// chord mode でコード進行を引き直すたびに音色も変えるための入口。
    /// 呼び出し側は差し替え後に `sender.prepare()` を走らせること（音色ロード中は
    /// 再生が止まる）。
    pub fn randomize_patches(
        &mut self,
        now: Instant,
        patches: &[(String, String)],
    ) -> Vec<GridScheduledMessage> {
        for instance in &mut self.instances {
            if let Some(index) = random_index(patches.len()) {
                instance.patch = Some(patches[index].0.clone());
            }
        }
        self.take_silence_messages(now)
    }

    /// patch を据え置いたまま、note number / patternだけを引き直す。
    ///
    /// 音色ロード（全 instance ぶんの patch prepare）が走らないので、呼んでも
    /// 再生が途切れない。realtime play の連続性を保ったまま鳴る内容だけを
    /// 変え続けたいとき（ジッタ・note off 漏れ・バッファ追従の検証）に使う。
    ///
    /// 返した note off は必ず送ること。`randomize_all` と違って音色切替の
    /// `stop_live_all()` が後ろに続かないため、送らないと音が鳴りっぱなしになる。
    pub fn randomize_keeping_patches(&mut self, now: Instant) -> Vec<GridScheduledMessage> {
        self.randomize_instances(
            &[],
            CycleRandom {
                patch: false,
                ..CycleRandom::ALL
            },
        );
        self.take_silence_messages(now)
    }

    /// 全instance/laneを引き直す。`patches` が空なら patch は据え置く。
    ///
    /// chord mode 中は引き直した note を現在のコードへ寄せ直すので、コードから
    /// 外れた音は出ない。
    fn randomize_instances(&mut self, patches: &[(String, String)], policy: CycleRandom) {
        let chord = self.chord.clone();
        let patterns = self.draw_pattern_combination(policy, chord.is_some());
        self.drawn = randomize_instance_slice(
            &mut self.instances,
            patches,
            policy,
            chord.as_ref(),
            patterns,
        );
        self.refresh_lane_display_patterns();
    }

    /// 鳴っている音を止める note off を、送信済みの先読みぶんより後ろへ置いて返す。
    pub(super) fn take_silence_messages(&mut self, now: Instant) -> Vec<GridScheduledMessage> {
        let ahead = self.silence_ahead(now);
        let timeline_seconds = self.silence_timeline_seconds();
        self.silence_sounding()
            .into_iter()
            .map(|(instance_id, message)| GridScheduledMessage {
                instance_id,
                ahead,
                timeline_seconds,
                message,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
