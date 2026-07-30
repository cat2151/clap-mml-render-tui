use std::time::Instant;

use rand::Rng;

use cmrt_tui_core::random::random_index;

use super::{GridScheduledMessage, GridState, StepDuration, GRID_STEPS};

/// ランダム生成に使う note number の範囲（C2〜C6）。
const RANDOM_NOTE_MIN: u8 = 36;
const RANDOM_NOTE_MAX: u8 = 84;
/// 1セルが note on になる確率。密になりすぎず、無音にもなりにくい値。
const CELL_ON_RATIO: f64 = 0.25;

impl GridState {
    /// patch 一覧の非同期読み込み後、まだ未設定の row だけへ音色を割り当てる。
    pub fn fill_missing_patches(&mut self, patches: &[(String, String)]) -> usize {
        let mut assigned = 0;
        for row in &mut self.rows {
            if row.patch.is_none() {
                if let Some(index) = random_index(patches.len()) {
                    row.patch = Some(patches[index].0.clone());
                    assigned += 1;
                }
            }
        }
        assigned
    }

    /// 全行の patch / note number / 音長と、全セルの note on 有無を引き直す。
    ///
    /// 引き直す前に鳴っていた音の note off を返す（鳴りっぱなしを防ぐ）。
    /// 先読みで送信済みの note on より後ろへ置くため、note off には猶予が乗る。
    /// `patches` が空（読み込み中・エラー）のときは patch だけ据え置く。
    pub fn randomize_all(
        &mut self,
        now: Instant,
        patches: &[(String, String)],
    ) -> Vec<GridScheduledMessage> {
        self.randomize_rows(patches);
        self.take_silence_messages(now)
    }

    /// patch を据え置いたまま、note number / 音長 / セルだけを引き直す。
    ///
    /// 音色ロード（全 instance ぶんの patch prepare）が走らないので、呼んでも
    /// 再生が途切れない。realtime play の連続性を保ったまま鳴る内容だけを
    /// 変え続けたいとき（ジッタ・note off 漏れ・バッファ追従の検証）に使う。
    ///
    /// 返した note off は必ず送ること。`randomize_all` と違って音色切替の
    /// `stop_live_all()` が後ろに続かないため、送らないと音が鳴りっぱなしになる。
    pub fn randomize_keeping_patches(&mut self, now: Instant) -> Vec<GridScheduledMessage> {
        // patch 一覧を空で渡すと patch だけ据え置かれる（`randomize_rows` 参照）。
        self.randomize_rows(&[]);
        self.take_silence_messages(now)
    }

    /// 全行を引き直す。`patches` が空なら patch は据え置く。
    fn randomize_rows(&mut self, patches: &[(String, String)]) {
        let mut rng = rand::thread_rng();
        for row in &mut self.rows {
            if let Some(index) = random_index(patches.len()) {
                row.patch = Some(patches[index].0.clone());
            }
            row.note = rng.gen_range(RANDOM_NOTE_MIN..=RANDOM_NOTE_MAX);
            row.duration = if rng.gen_bool(0.5) {
                StepDuration::Sixteenth
            } else {
                StepDuration::Quarter
            };
            for step in 0..GRID_STEPS {
                row.cells[step] = rng.gen_bool(CELL_ON_RATIO);
            }
        }
    }

    /// 鳴っている音を止める note off を、送信済みの先読みぶんより後ろへ置いて返す。
    fn take_silence_messages(&mut self, now: Instant) -> Vec<GridScheduledMessage> {
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
