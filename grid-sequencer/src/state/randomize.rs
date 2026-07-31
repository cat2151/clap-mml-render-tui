use std::time::Instant;

use rand::Rng;

use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::{patches::patch_matches_categories, random::random_index};

use super::{GridScheduledMessage, GridState, StepDuration, GRID_STEPS};
use crate::GridVoicingLookup;

/// ランダム生成に使う note number の範囲（C2〜C6）。
const RANDOM_NOTE_MIN: u8 = 36;
const RANDOM_NOTE_MAX: u8 = 84;
/// 1セルが note on になる確率。密になりすぎず、無音にもなりにくい値。
const CELL_ON_RATIO: f64 = 0.25;

/// 和音に使える patch を1つ引く。当たりは「指定カテゴリに属し、かつ poly と
/// 判明している」もの。
///
/// mono patch では和音が最後の1音しか鳴らないため、chord mode の行には使えない。
/// 未判定（`None`）も外れ扱いにするので、voicing キャッシュが空だと何も引けない。
/// `categories` が空ならカテゴリでは絞らない。
pub fn pick_chord_patch(
    patches: &[(String, String)],
    voicing: &dyn GridVoicingLookup,
    categories: &[String],
) -> Option<String> {
    // 当たりが薄いときに引き直しで粘るより、先に候補を絞ったほうが確実で速い。
    let candidates = patches
        .iter()
        .filter(|(display, lower)| {
            patch_matches_categories(lower, categories)
                && voicing.cached_voicing(display) == Some(PatchVoicing::Poly)
        })
        .collect::<Vec<_>>();
    let index = random_index(candidates.len())?;
    Some(candidates[index].0.clone())
}

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

    /// 全行の patch だけを引き直す。譜面（note / 音長 / セル）は据え置く。
    ///
    /// chord mode でコード進行を引き直すたびに音色も変えるための入口。
    /// 呼び出し側は差し替え後に `sender.prepare()` を走らせること（音色ロード中は
    /// 再生が止まる）。
    pub fn randomize_patches(
        &mut self,
        now: Instant,
        patches: &[(String, String)],
    ) -> Vec<GridScheduledMessage> {
        for row in &mut self.rows {
            if let Some(index) = random_index(patches.len()) {
                row.patch = Some(patches[index].0.clone());
            }
        }
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
    ///
    /// chord mode 中は引き直した note を現在のコードへ寄せ直すので、コードから
    /// 外れた音は出ない。
    fn randomize_rows(&mut self, patches: &[(String, String)]) {
        let mut rng = rand::thread_rng();
        for row in &mut self.rows {
            if let Some(index) = random_index(patches.len()) {
                row.patch = Some(patches[index].0.clone());
            }
            row.base_note = rng.gen_range(RANDOM_NOTE_MIN..=RANDOM_NOTE_MAX);
            row.note = row.base_note;
            row.duration = if rng.gen_bool(0.5) {
                StepDuration::Sixteenth
            } else {
                StepDuration::Quarter
            };
            for step in 0..GRID_STEPS {
                row.cells[step] = rng.gen_bool(CELL_ON_RATIO);
            }
        }
        self.apply_chord_to_rows();
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
