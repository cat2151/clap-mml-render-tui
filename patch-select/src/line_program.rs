//! 行をまるごと鳴らすときに運ぶ型。
//!
//! 行をイベント列へ変換する関数はここに無い（MML・chord を解釈する側が持つ）。
//! 「行をどう鳴らすか」（1 回だけか鳴らし続けるか、MIDI filter を重ねるか）も
//! [`LineProgram`] としてここに置く。イベント列と鳴らし方は必ず対で運ばれ、
//! 受け取る sender が両方を見て初めて 1 回の演奏になるため。

use cmrt_chord::TimedMidiEvent;

/// 直近に行を演奏した結果。入力欄の下に出す。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum LineStatus {
    /// まだ演奏していない、または空行にいる。
    #[default]
    Idle,
    Played {
        /// chord2mml がコード表記として受け付けたか。
        from_chord: bool,
        /// 鳴らすノート数（和音は構成音ぶん数える）。
        note_count: usize,
    },
    Error(String),
}

/// 1 行ぶんの演奏内容。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinePerformance {
    /// 時刻順に並んだ note on / note off。空なら「止めるだけ」の意味になる。
    pub events: Vec<TimedMidiEvent>,
    /// 1 周の長さ。繰り返すとき、次の周をこれだけ後ろへずらす。
    ///
    /// **罠: これは `cmrt_chord::TimedPerformance::duration_seconds` そのままで、
    /// 「最後のイベントまで」しか測っていない。行末の休符は落ちる。**
    /// ループが詰まって聞こえたらここが原因。
    pub loop_seconds: f64,
}

impl LinePerformance {
    /// 鳴らすものが無い。受け取った側は前の演奏を止めるだけになる。
    pub fn silent() -> Self {
        Self::default()
    }

    pub fn is_silent(&self) -> bool {
        self.events.is_empty()
    }
}

/// 演奏へ重ねる MIDI filter の ON/OFF。
///
/// 実際に掛けるのは sender 側。ここは「掛けてほしいか」だけを運ぶ。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FilterSettings {
    /// CC1 modulation を LFO で重ねる。
    pub modulation: bool,
    /// note on の velocity を LFO の値で乗っ取る（MML の `v` 指定を無視する）。
    pub velocity: bool,
}

/// 「この行をどう鳴らしてほしいか」ひとまとまり。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineProgram {
    pub performance: LinePerformance,
    /// 鳴らし終わっても止めず、同じ内容を継ぎ足して鳴らし続ける。
    pub repeat: bool,
    pub filters: FilterSettings,
}

impl LineProgram {
    /// 1 回だけ鳴らす（filter なし）。演奏設定がまだ無い経路はこれを使う。
    pub fn once(performance: LinePerformance) -> Self {
        Self {
            performance,
            repeat: false,
            filters: FilterSettings::default(),
        }
    }

    /// 鳴らすものが無い。前の演奏を止めるだけの指示になる。
    pub fn silent() -> Self {
        Self::once(LinePerformance::silent())
    }

    pub fn is_silent(&self) -> bool {
        self.performance.is_silent()
    }

    pub fn events(&self) -> &[TimedMidiEvent] {
        &self.performance.events
    }
}
