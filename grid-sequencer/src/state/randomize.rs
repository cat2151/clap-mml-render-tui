use std::time::Instant;

use rand::RngExt;

use cmrt_tui_core::random::random_index;

use super::{drum, GridInstance, GridScheduledMessage, GridState, NotePattern, GRID_STEPS};

/// ランダム生成に使う note number の範囲（C2〜C6）。
const RANDOM_NOTE_MIN: u8 = 36;
const RANDOM_NOTE_MAX: u8 = 84;
/// Rest位置でnote eventを開始する確率。密になりすぎず、無音にもなりにくい値。
const CELL_ON_RATIO: f64 = 0.25;
/// 1stepを多めにしつつ、2/4step noteも生成する重み付き候補。
const NOTE_LENGTHS: [usize; 6] = [1, 1, 1, 2, 2, 4];

/// instance群を丸ごと引き直す。`patches` が空なら patch だけ据え置く。
///
/// `GridState` の外へ出してあるのは、chord mode が「鳴っている grid を触らずに
/// 次サイクルを抽選する」ために、複製したinstance群へ同じ抽選をかけるため。
pub fn randomize_instance_slice(instances: &mut [GridInstance], patches: &[(String, String)]) {
    let mut rng = rand::rng();
    for instance in instances {
        if let Some(index) = random_index(patches.len()) {
            instance.patch = Some(patches[index].0.clone());
        }
        // drum 行は音高も譜面も専用（[`super::drum`]）。音高を引き直すと打楽器の音色
        // そのものが変わり、汎用の譜面を当てるとリズムでなくなる。
        if let Some(role) = instance.drum {
            drum::randomize_drum_pattern(instance, role, &mut rng);
            continue;
        }
        for lane in &mut instance.lanes {
            lane.base_note = rng.random_range(RANDOM_NOTE_MIN..=RANDOM_NOTE_MAX);
            lane.pattern = random_pattern(&mut rng);
        }
    }
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
        self.randomize_instances(patches);
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
        // patch 一覧を空で渡すと patch だけ据え置かれる（`randomize_instances` 参照）。
        self.randomize_instances(&[]);
        self.take_silence_messages(now)
    }

    /// 全instance/laneを引き直す。`patches` が空なら patch は据え置く。
    ///
    /// chord mode 中は引き直した note を現在のコードへ寄せ直すので、コードから
    /// 外れた音は出ない。
    fn randomize_instances(&mut self, patches: &[(String, String)]) {
        randomize_instance_slice(&mut self.instances, patches);
        self.refresh_lane_display_patterns();
    }

    /// 鳴っている音を止める note off を、送信済みの先読みぶんより後ろへ置いて返す。
    pub(super) fn take_silence_messages(&mut self, now: Instant) -> Vec<GridScheduledMessage> {
        let ahead = self.silence_ahead(now);
        self.silence_sounding()
            .into_iter()
            .map(|(instance_id, message)| GridScheduledMessage {
                instance_id,
                ahead,
                message,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
