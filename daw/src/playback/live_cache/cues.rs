//! その小節で「どの行を、どの instance の、どのスロットで鳴らすか」の組み立て。
//!
//! 実サーバーもファイルシステムも触らない。送信は [`super::send`]、ログは
//! [`super::measure_log`] が持つ。

use std::path::PathBuf;

use cmrt_core::cache_wav::SLOT_COUNT;
use cmrt_realtime_play::{InstanceId, TimelineMidiEvent};

use crate::live_instance::live_instance_for_grid_row;
use crate::tracks::FIRST_PLAYABLE_TRACK;

/// スロット 0 を鳴らす note number（C4）。スロット `n` は `BASE + n`。
///
/// cache-player は `slot_for_note(note) = note % SLOT_COUNT` でスロットを選ぶ
/// （play server 側 `cache-player/src/slots.rs`）。`60 % 2 == 0` なので、
/// **音高を見ていなかった頃の「常に note 60」はスロット 0** に落ちる＝後方互換が取れる。
const CACHE_PLAYER_BASE_NOTE: u8 = 60;
/// note on の velocity。cache-player は強弱を持たないのでログで紛れない固定値。
const CACHE_PLAYER_VELOCITY: u8 = 100;

/// 小節 index からスロット index を決める。
///
/// **`measure_index % SLOT_COUNT`。** 隣り合う小節が必ず別スロットになるので、
/// 小節 N を鳴らしている最中に小節 N+1 を載せても、鳴っている音のスロットを壊さない。
/// 載せ先とそれを鳴らす note number の対応は cache-player 側と揃っている必要があるので、
/// **ここと [`note_on_events`] の 2 か所だけで決めること。**
pub(crate) fn measure_slot(measure_index: usize) -> usize {
    measure_index % SLOT_COUNT
}

/// ある小節で、ある行のキャッシュを鳴らすために送る 1 組。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiveCacheCue {
    pub(crate) row: usize,
    pub(crate) instance: InstanceId,
    pub(crate) wav: PathBuf,
}

/// 1 小節ぶんの送信内容と、送らなかった行の内訳。
///
/// 送らなかった行を捨てずに持つのは、**無音が意図どおりか**をログで確かめるため。
/// 「キャッシュがまだ無い（`silent_rows`）」と「instance が足りない
/// （`rows_over_instance_limit`）」は原因が別物なので分けてある。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct MeasureLiveCues {
    pub(crate) cues: Vec<LiveCacheCue>,
    pub(crate) silent_rows: Vec<usize>,
    pub(crate) rows_over_instance_limit: Vec<usize>,
}

/// 1 小節ぶんに live 経路へ送るものを組み立てる。
///
/// `ready_cache_wav` は「その行のキャッシュ WAV が**存在するなら**その絶対パス」を返す。
/// 実ファイルを見るのは呼び出し側の責務にしてあるので、この関数は実サーバーも
/// ファイルシステムも無しで単体テストできる。
pub(crate) fn measure_live_cues(
    tracks: usize,
    ready_cache_wav: impl Fn(usize) -> Option<PathBuf>,
) -> MeasureLiveCues {
    let mut cues = MeasureLiveCues::default();
    for row in FIRST_PLAYABLE_TRACK..tracks {
        let Some(instance) = live_instance_for_grid_row(row) else {
            cues.rows_over_instance_limit.push(row);
            continue;
        };
        match ready_cache_wav(row) {
            Some(wav) => cues.cues.push(LiveCacheCue { row, instance, wav }),
            None => cues.silent_rows.push(row),
        }
    }
    cues
}

/// 小節の頭で全 track を一斉に鳴らすための note on を 1 バッチに組み立てる。
///
/// **発音位置は `at_seconds`（timeline 原点からの絶対秒）で指定する。** サーバーは
/// これをサンプル位置へ丸めて予約するので、送った時刻がぶれても発音位置は動かない
/// （`offset_frames: 0` の生 live イベントで送っていたころは「届いたオーディオ
/// ブロックの頭」で鳴っていたため、小節ごとに −42.7〜+21.3ms ずれていた）。
///
/// note number は `CACHE_PLAYER_BASE_NOTE + slot`。cache-player は音高でスロットを
/// 選ぶので、**先読みで載せた先と同じスロットを指していないと前の小節が鳴る**。
/// `slot` は必ず [`measure_slot`] から取ること。
///
/// バッチの上限は `MAX_MIDI_MESSAGES`（128）で、cue は最大 16
/// （[`crate::live_instance::MAX_LIVE_TRACKS`]）なので分割は要らない。
pub(crate) fn note_on_events(
    cues: &[LiveCacheCue],
    slot: usize,
    timeline_id: u64,
    at_seconds: f64,
) -> Vec<TimelineMidiEvent> {
    let note = CACHE_PLAYER_BASE_NOTE + slot as u8;
    cues.iter()
        .map(|cue| TimelineMidiEvent {
            timeline_id,
            instance_id: cue.instance,
            timeline_seconds: at_seconds,
            message: [0x90, note, CACHE_PLAYER_VELOCITY],
        })
        .collect()
}
