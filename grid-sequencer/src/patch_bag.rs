//! PATCH 欄の wheel が辿る、shuffle 済みの patch list。
//!
//! テトリスの 7-bag と同じ考え方で、候補を1周ぶん shuffle した袋を作り、使い切ったら
//! 次の袋を継ぎ足す。1周のあいだ同じ patch は2度出ないので、同じ音色を続けて引いたり
//! 特定の音色にいつまでも当たらなかったりしない。都度ランダムに引くのとはここが違う。
//!
//! wheel は list を送る操作なので、下で次、上で前へ戻る（[`ListDirection`]）。戻れる
//! ように引いた順序を `sequence` に残し、カーソルだけを動かす。list の先頭は袋から
//! 引いたものではなく画面に出ていた patch そのものなので、上へ戻しきれば必ず元の音色
//! へ帰れる。

use cmrt_patches::PatchRole;
use rand::seq::SliceRandom;

use crate::ListDirection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PatchBag {
    /// この袋を作ったときの行の用途。chord mode の on/off で変わる。
    role: PatchRole,
    /// 袋を作る元。用途か候補が変われば袋ごと作り直す判定に使う。
    candidates: Vec<String>,
    /// 引いた順。`cursor` がこの中を前後する。
    sequence: Vec<String>,
    cursor: usize,
    /// 今の袋の残り。空になったら shuffle し直して継ぎ足す。
    remaining: Vec<String>,
}

impl PatchBag {
    /// 画面に出ている patch を list の先頭に据えて袋を作る。
    ///
    /// `current` が候補に含まれていなくてもよい。「wheel を回す前に鳴っていた音色」
    /// へ戻れることのほうが、list の中身が候補だけで揃っていることより大事。
    pub(crate) fn new(role: PatchRole, candidates: Vec<String>, current: Option<&str>) -> Self {
        Self {
            role,
            candidates,
            sequence: current.map(str::to_string).into_iter().collect(),
            cursor: 0,
            remaining: Vec::new(),
        }
    }

    /// 同じ用途・同じ候補で作った袋か。違えば呼び出し側が作り直す。
    pub(crate) fn matches(&self, role: PatchRole, candidates: &[String]) -> bool {
        self.role == role && self.candidates == candidates
    }

    /// カーソルを1つ送って、適用する patch を返す。
    ///
    /// 先頭より前へは戻らない。戻りきった状態でさらに上へ回すと先頭を返し続けるので、
    /// 呼び出し側では「同じ patch なので何もしない」に落ちる。
    pub(crate) fn advance(&mut self, direction: ListDirection) -> Option<&str> {
        match direction {
            ListDirection::Next => {
                if self.cursor + 1 < self.sequence.len() {
                    // 一度上へ戻ったぶんは引き直さず、同じ並びを辿り直す。
                    self.cursor += 1;
                } else {
                    let patch = self.draw()?;
                    self.sequence.push(patch);
                    self.cursor = self.sequence.len() - 1;
                }
            }
            ListDirection::Prev => {
                if self.sequence.is_empty() {
                    return None;
                }
                self.cursor = self.cursor.saturating_sub(1);
            }
        }
        self.sequence.get(self.cursor).map(String::as_str)
    }

    /// 袋から1つ取り出す。空なら shuffle し直して補充する。
    fn draw(&mut self) -> Option<String> {
        if self.remaining.is_empty() {
            self.remaining = self.candidates.clone();
            self.remaining.shuffle(&mut rand::rng());
        }
        // 袋の継ぎ目や、先頭に据えた patch とぶつかると wheel が効かなく見える。その
        // ときだけ隣と入れ替える。捨てないので「1周に1回ずつ」は崩れない。
        if self.remaining.len() >= 2 && self.sequence.last() == self.remaining.last() {
            let last = self.remaining.len() - 1;
            self.remaining.swap(last, last - 1);
        }
        self.remaining.pop()
    }
}

#[cfg(test)]
mod tests;
