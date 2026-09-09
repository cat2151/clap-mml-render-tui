//! Chord Chart 画面と TuiApp を接続する glue。
//!
//! 画面ロジック（状態・キー処理・描画）は `cmrt-chord-chart` crate に閉じている。
//! app 側に残るのは「キーを渡す」「変更されたら保存する」だけ。
//!
//! **保存を app 側に置く理由**: crate 側はログを持たない（音も鳴らさないので
//! 失敗を音で気づくこともない）。保存の失敗をログ 1 行にとどめて画面を落とさない、
//! という判断は app の責務にする。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;

use crossterm::event::KeyEvent;

use crate::tui::chord_chart::ChordChartAction;
use crate::tui::TuiApp;

impl TuiApp<'_> {
    /// キー 1 つを画面へ渡し、保存だけをここで済ませる。
    /// [`ChordChartAction::Quit`]（`q`）はランタイムがループを抜けるので、そのまま返す。
    pub(in crate::tui) fn handle_chord_chart_key_event(
        &mut self,
        key: KeyEvent,
    ) -> ChordChartAction {
        let action = self.chord_chart.handle_key_event(key);
        // デバウンス禁止。変更が起きたその場で書く（ファイルは数 KB）。
        if action == ChordChartAction::SongChanged {
            self.save_chord_chart();
        }
        action
    }

    /// 画面へ入るときに 1 度だけ呼ぶ。保存ファイルが読めなかったときの自動抽選
    /// （`g` 1 回ぶん）はここで走る。
    ///
    /// **起動時ではなくここで引く理由**: カタログは遅延取得で、キャッシュがまだ無い
    /// 初回は取得完了まで待つ（上限 20 秒）。`TuiApp::new` で引くと、chord chart を
    /// 開かない人まで起動をその時間止められる。
    ///
    /// 抽選できた曲はその場で保存する。保存しないと、次の起動でまた別の進行が出る。
    pub(in crate::tui) fn enter_chord_chart(&mut self) {
        if self.chord_chart.enter() == ChordChartAction::SongChanged {
            self.save_chord_chart();
        }
    }

    /// 前回 chord chart 画面で終了していた場合の入り口。
    /// `switch_to_primary_screen` を通らない経路なので、run() の冒頭で一度だけ呼ぶ。
    pub(in crate::tui) fn enter_restored_chord_chart(&mut self) {
        if self.active_screen == crate::screen_switch::PrimaryScreen::ChordChart {
            self.enter_chord_chart();
        }
    }

    /// 画面を離れるときの保存。キーごとの保存で足りているはずだが、
    /// 取りこぼしがあってもここで拾う。
    pub(in crate::tui) fn save_chord_chart(&self) {
        if let Err(error) = crate::tui::chord_chart::save_song(&self.chord_chart.song) {
            crate::logging::global_log_sink(&format!(
                "chord-chart: event=save-failed error=\"{error}\""
            ));
        }
    }
}

/// Chord Chart 画面へ渡すコード進行カタログの供給元を組み立てる。
///
/// カタログの取得・キャッシュは app の責務なので、画面 crate へは関数だけを貸す。
/// **遅延評価**にしてあるのは、キャッシュがまだ無い初回に `fetch` が取得完了を待つため
/// （画面を開くたびに待たされないよう、`g` / `r` を押したときにだけ引く）。
///
/// その初回の待ち時間を 1 行だけログへ出す。体感で許容できるかの判断は人間だが、
/// **秒数そのものは測って残す**（次に調べるとき、また同じ計測を仕込まずに済む）。
pub(in crate::tui) fn chord_chart_catalog_source(
    fetch: impl Fn() -> Vec<String> + Send + Sync + 'static,
    log: impl Fn(&str) + Send + Sync + 'static,
) -> Arc<dyn Fn() -> Vec<String> + Send + Sync> {
    let logged = AtomicBool::new(false);
    Arc::new(move || {
        let started = Instant::now();
        let progressions = fetch();
        // 2 回目以降はキャッシュ済みで一瞬なので、記録するのは初回だけ。
        if !logged.swap(true, Ordering::Relaxed) {
            log(&format!(
                "chord-chart: event=catalog-first-load elapsed_ms={} count={}",
                started.elapsed().as_millis(),
                progressions.len()
            ));
        }
        progressions
    })
}

/// production が実際に渡す供給元。`ChordProgressionSource`（取得とキャッシュは app の責務）
/// から degrees だけを取り出して貸す。
///
/// **`TuiApp::new` へ直書きせず関数にしてある理由**: 初回取得の待ち時間は
/// 「体感で許容できるか」を人間が決める材料だが、その秒数は実経路を通さないと測れない。
/// ここを名前のある関数にしておけば、画面を起動せずに実測できる
/// （`the_cold_catalog_first_load_is_measured_through_the_real_source`）。
pub(in crate::tui) fn chord_chart_catalog_source_from(
    source: crate::chord_progression_source::ChordProgressionSource,
    log: impl Fn(&str) + Send + Sync + 'static,
) -> Arc<dyn Fn() -> Vec<String> + Send + Sync> {
    chord_chart_catalog_source(
        move || {
            source
                .catalog()
                .entries()
                .iter()
                .map(|entry| entry.degrees.clone())
                .collect()
        },
        log,
    )
}
