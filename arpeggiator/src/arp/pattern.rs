//! アルペジオの音型。
//!
//! ここが知っているのは「何番目の声部を、どの順で鳴らすか」だけ。実際の音高への
//! 解決（コード構成音なのかスケールなのか）は呼び出し側の責務にしてある。
//!
//! 声部は `0` が最低音、`voice_count - 1` が最高音。

/// アルペジオの音型。
///
/// 複数オクターブへ広げる `oct2` / `oct3` のようなレンジ指定はまだ持たない。
/// 声部集合そのものを呼び出し側が決める設計なので、レンジを足すなら
/// 「呼び出し側が声部を増やす」か「この enum にレンジ修飾を足す」かの選択になる。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArpPattern {
    /// 0,1,2,3
    #[default]
    Up,
    /// 3,2,1,0
    Down,
    /// 0,1,2,3,2,1 — 折り返しで端を重ねない
    UpDown,
    /// 3,2,1,0,1,2
    DownUp,
    /// 0,1,2,3,3,2,1,0 — 折り返しで端を重ねる
    UpDownHold,
    /// 0,3,1,2 — 外側から内側へ
    Converge,
    /// 1,2,0,3 — 内側から外側へ
    Diverge,
    /// 0,3,1,3,2,3 — 最高音と交互。三和音なら最高音が root のオクターブ上になる
    Octave,
    /// 周期を持たず、1音ごとに声部を引き直す
    Random,
}

impl ArpPattern {
    /// wheel の種別送りで巡回する順序。
    pub const ALL: [ArpPattern; 9] = [
        Self::Up,
        Self::Down,
        Self::UpDown,
        Self::DownUp,
        Self::UpDownHold,
        Self::Converge,
        Self::Diverge,
        Self::Octave,
        Self::Random,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::UpDown => "UpDown",
            Self::DownUp => "DownUp",
            Self::UpDownHold => "UpDownHold",
            Self::Converge => "Converge",
            Self::Diverge => "Diverge",
            Self::Octave => "Octave",
            Self::Random => "Random",
        }
    }

    /// [`Self::ALL`] の次の音型。末尾からは先頭へ戻る。
    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    /// [`Self::ALL`] の前の音型。先頭からは末尾へ戻る。
    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    /// 音型1周期ぶんの voice index。周期を持たない [`Self::Random`] は `None`。
    ///
    /// `voice_count` が 0 のときは空を返す。
    pub fn voice_sequence(self, voice_count: usize) -> Option<Vec<usize>> {
        if self == Self::Random {
            return None;
        }
        if voice_count == 0 {
            return Some(Vec::new());
        }
        let top = voice_count - 1;
        Some(match self {
            Self::Up => (0..voice_count).collect(),
            Self::Down => (0..voice_count).rev().collect(),
            Self::UpDown => (0..voice_count).chain((1..top).rev()).collect(),
            Self::DownUp => (0..voice_count).rev().chain(1..top).collect(),
            Self::UpDownHold => (0..voice_count).chain((0..voice_count).rev()).collect(),
            Self::Converge => converge(voice_count),
            Self::Diverge => diverge(voice_count),
            Self::Octave => octave(voice_count),
            Self::Random => unreachable!("Random returned early"),
        })
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|pattern| *pattern == self)
            .expect("ALL contains every variant")
    }
}

/// 外側から内側へ。奇数個なら中央が最後に1回だけ出る。
fn converge(voice_count: usize) -> Vec<usize> {
    let mut voices = Vec::with_capacity(voice_count);
    let (mut low, mut high) = (0, voice_count - 1);
    while low < high {
        voices.push(low);
        voices.push(high);
        low += 1;
        high -= 1;
    }
    if low == high {
        voices.push(low);
    }
    voices
}

/// 内側から外側へ。奇数個なら中央が最初に1回だけ出る。
fn diverge(voice_count: usize) -> Vec<usize> {
    let mut voices = Vec::with_capacity(voice_count);
    let mut low = (voice_count - 1) / 2;
    let mut high = voice_count / 2;
    loop {
        voices.push(low);
        if high != low {
            voices.push(high);
        }
        if low == 0 {
            break;
        }
        low -= 1;
        high += 1;
    }
    voices
}

/// 最高音を挟みながら下から順に上がる。声部が1つしかなければそれだけを返す。
fn octave(voice_count: usize) -> Vec<usize> {
    let top = voice_count - 1;
    if top == 0 {
        return vec![0];
    }
    (0..top).flat_map(|voice| [voice, top]).collect()
}

#[cfg(test)]
mod tests;
