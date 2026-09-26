//! 小節のWAVロードと起動時のロード結果ログ。
use super::{
    measure_live_cues, measure_slot, prepare_measure_cues, LiveCachePlayLoop, PreloadedMeasure,
};

impl LiveCachePlayLoop {
    /// 1 小節ぶんのキャッシュ WAV を、その小節のスロットへ載せる。
    ///
    /// 載せ先は `measure_index % SLOT_COUNT` に決まっているので、**隣り合う小節は
    /// 必ず別スロット**になる。鳴っている voice は自分が握った音源を鳴らし続けるので
    /// （`clap-mml-play-server` の `docs/adr/0018`）、ここで差し替えても前の小節の余韻は切れない。
    pub(super) fn load_measure(&self, measure_index: usize) -> PreloadedMeasure {
        self.load_measure_reporting(measure_index, false)
    }

    /// `report_startup` が true のときだけ、載せ終えた本数を
    /// [`super::DawPlaybackStartupState`] へ報告する（＝「音が鳴るまで」overlay へ出す）。
    ///
    /// **報告するのは演奏開始の 1 小節目だけ。** 2 小節目以降は鳴っている最中の
    /// 先読みなので、進捗を出すと「鳴っているのに読み込み中」に見える。
    pub(super) fn load_measure_reporting(
        &self,
        measure_index: usize,
        report_startup: bool,
    ) -> PreloadedMeasure {
        let cues = measure_live_cues(self.tracks, |row| {
            (self.ready_cache_wav)(measure_index, row)
        });
        if report_startup {
            self.startup.begin_first_measure(cues.cues.len());
            if let Some(progress) = &self.autoplay_progress {
                progress.log(
                    &self.log_lines,
                    format!(
                        "開始小節 M{} のWAVロードを開始します。対象={}",
                        measure_index + 1,
                        cues.cues.len()
                    ),
                );
            }
        }
        let measure = prepare_measure_cues(
            &self.play_server,
            measure_index,
            measure_slot(measure_index),
            cues,
            &self.log_lines,
            &mut |loaded| {
                if report_startup {
                    self.startup.note_measure_loaded(loaded);
                }
            },
        );
        if report_startup {
            if let Some(progress) = &self.autoplay_progress {
                progress.log(
                    &self.log_lines,
                    format!(
                        "開始小節のWAVロード処理が終了しました。成功={}、失敗={}",
                        measure.prepared.len(),
                        measure.cues.cues.len() - measure.prepared.len()
                    ),
                );
            }
        }
        measure
    }
}
