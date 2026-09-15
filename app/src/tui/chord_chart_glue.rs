//! Chord Chart 画面と TuiApp を接続する glue。
//!
//! 画面ロジック（状態・キー処理・描画）は `cmrt-chord-chart` crate に閉じている。
//! app 側は「キーを渡す」「host editor を開く」「変更されたら保存する」「preview 要求を
//! 回収する」に加え、Chord Chart の canonical patch と sender の実演奏状態を所有する。
//!
//! **保存も preview も app 側に置く理由**: crate 側はログを持たないし、音を鳴らす
//! 手段（`MmlOverlaySender`）も知らない。保存の失敗をログ 1 行にとどめて画面を
//! 落とさない、鳴らす 1 行をどう組み立てるか、という判断は app の責務にする。
//! 画面 crate は「何を鳴らすべきか」を要求として立てるところまで（ADR 0020）。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;

use crossterm::event::KeyEvent;

use cmrt_mml_overlay::line_play::LineStatus;

use crate::tui::chord_chart::{ChordChartAction, PreviewRequest, SectionId, Song};
use crate::tui::TuiApp;

mod bass_patch;
mod preview;
mod voicing;

#[cfg(test)]
pub(in crate::tui) use preview::preview_line;
pub(in crate::tui) use preview::{
    key_token, preview_command_log_line, preview_command_sounding, preview_play_log_line,
    preview_request_log_line, ChordChartPreview,
};

impl TuiApp<'_> {
    /// キー 1 つを画面へ渡し、host action の実行と保存をここで済ませる。
    /// [`ChordChartAction::Quit`]（`q`）はランタイムがループを抜けるので、そのまま返す。
    pub(in crate::tui) fn handle_chord_chart_key_event(
        &mut self,
        key: KeyEvent,
    ) -> ChordChartAction {
        // `Shift+P` / `Space` のトグルが「鳴っていたら止める」を決められるように、
        // キーを渡す**前**に最新の答えを書き戻す。時間で終わる 1 回きりの演奏なので、
        // 押した瞬間に測り直さないと「もう鳴っていないのに止める」空打ちが出る。
        self.refresh_chord_chart_preview_sounding();
        let action = self.chord_chart.handle_key_event(key);
        // デバウンス禁止。変更が起きたその場で書く（ファイルは数 KB）。
        // 曲が変わった＝ degrees が変わりうるので、chord の範囲の写しもここで作り直す
        // （preview を回収するより先に。要求の中の番号はこの写しから決まる）。
        match action {
            ChordChartAction::SongChanged => {
                self.save_chord_chart();
                self.refresh_chord_chart_chord_ranges();
            }
            ChordChartAction::EditDegrees(section_id) => {
                self.open_chord_chart_degrees_overlay(section_id);
            }
            ChordChartAction::PreviewSettingChanged => self.save_history_state(),
            ChordChartAction::Quit => self.stop_chord_chart_preview(),
            ChordChartAction::Continue => {}
        }
        self.drain_chord_chart_preview();
        action
    }

    /// 「各 section の行内で、どこからどこまでが 1 つの chord か」を画面へ書き戻す。
    /// chord が何個あるかも、画面はこの写しの長さから引く。
    ///
    /// **画面 crate は degrees を解釈しないので自分では切れない**（ADR 0020）。
    /// `preview_sounding` と同じで、答えを持っているのは glue だけ。
    ///
    /// 呼ぶのは**曲が変わったときと画面へ入ったときだけ**。カーソルを動かしても
    /// degrees は変わらないので、キーごとに切り直す理由が無い
    /// （1 section あたり 90 µs 未満だが、押すたびに全 section を切らない）。
    pub(in crate::tui) fn refresh_chord_chart_chord_ranges(&mut self) {
        let ranges = chord_ranges(&self.chord_chart.song);
        self.chord_chart.set_chord_ranges(ranges);
    }

    /// 立っている preview 要求を回収し、鳴らす。
    ///
    /// **回収は画面 crate の外＝ここでしかできない**（音を鳴らす手段を持っているのは
    /// app 側だけで、`cmrt-chord-chart` は `MmlOverlaySender` を知らない）。
    ///
    /// 必ず消費すること。残したまま次のキーを処理すると、1 回のカーソル移動で
    /// 2 回鳴る要求が溜まる。
    ///
    /// 前の音を止めるのは sender の責務（`play_line` は鳴っているものを止めてから
    /// 積む）。無音要求は patch load を起こさない `stop` として送る。
    fn drain_chord_chart_preview(&mut self) {
        let Some(request) = self.chord_chart.take_preview() else {
            return;
        };
        crate::logging::global_log_sink(&preview_request_log_line(
            &request,
            self.chord_chart.bass_enabled(),
        ));
        self.play_or_defer_chord_chart_preview(request);
    }

    /// Bass の既定音色がまだ分からない間は Chord だけを先行再生せず、最新要求を保留する。
    fn play_or_defer_chord_chart_preview(&mut self, request: PreviewRequest) {
        let bass_patch = self.resolve_chord_chart_bass_patch();
        if self.chord_chart.bass_enabled()
            && !request.is_silent()
            && matches!(bass_patch, bass_patch::BassPatchResolution::Loading)
        {
            // 直前の preview が残っていれば止める。ただし、保留要求は stop のあとで載せる。
            self.stop_chord_chart_preview();
            self.deferred_chord_chart_preview = Some(request);
            self.chord_chart.error = Some("Bass patch catalog を読み込み中です".to_string());
            crate::logging::global_log_sink(
                "chord-chart: event=preview-command status=deferred reason=bass-catalog-loading",
            );
            return;
        }

        self.deferred_chord_chart_preview = None;
        let preview = self.chord_chart_preview_with_bass_patch(&request, bass_patch);
        crate::logging::global_log_sink(&preview_play_log_line(&preview));
        // 読めなかった理由は、鳴らせるかどうかに関係なく画面へ出す
        // （play server が上がっていないときも、理由は理由として見せる）。
        if let LineStatus::Error(reason) = &preview.status {
            self.chord_chart.error = Some(format!("鳴らせません: {reason}"));
        } else if let Some(reason) = &preview.bass_reason {
            self.chord_chart.error = Some(format!("Bassを鳴らせません: {reason}"));
        }
        // play server が上がっていなければ sender が無い。落とさず、何もしない。
        self.chord_chart_preview_command_id = self
            .mml_overlay_sender
            .as_ref()
            .map(|sender| sender.play_layers(preview.layers.clone()));
        crate::logging::global_log_sink(&preview_command_log_line(
            &preview,
            self.chord_chart_preview_command_id,
        ));
        self.refresh_chord_chart_preview_sounding();
    }

    /// 初期 catalog load が終わった frame で、保留していた最新 preview を自動再生する。
    pub(in crate::tui) fn pump_chord_chart_preview(&mut self) {
        if self.active_screen != crate::screen_switch::PrimaryScreen::ChordChart
            || self.mml_overlay.is_open()
            || self.deferred_chord_chart_preview.is_none()
        {
            return;
        }
        if matches!(
            self.resolve_chord_chart_bass_patch(),
            bass_patch::BassPatchResolution::Loading
        ) {
            return;
        }
        let request = self
            .deferred_chord_chart_preview
            .take()
            .expect("presence checked above");
        if self.chord_chart.error.as_deref() == Some("Bass patch catalog を読み込み中です")
        {
            self.chord_chart.error = None;
        }
        self.play_or_defer_chord_chart_preview(request);
    }

    /// 「preview がまだ鳴っているか」の答えを画面へ書き戻す。
    ///
    /// 画面 crate は音を持たないので自分では知り得ない（`set_preview_sounding` の
    /// doc を参照）。glue が preview を投げた直後と、キーを渡す直前に更新する。
    fn refresh_chord_chart_preview_sounding(&mut self) {
        let sounding = self.chord_chart_preview_sounding(Instant::now());
        self.chord_chart.set_preview_sounding(sounding);
    }

    /// Chord Chart が所有している layered preview を止め、画面側の記録も捨てる。
    ///
    /// layered timeline は Chord/Bass の両 instance を含むため、note off の済んだ
    /// one-shot でも sender の `stop` へ通す。これにより画面離脱・overlay open・
    /// `q` 終了のどの経路でも、server 側の共有 timeline と両 instance を止める。
    pub(in crate::tui) fn stop_chord_chart_preview(&mut self) {
        if let Some(sender) = &self.mml_overlay_sender {
            sender.stop();
        }
        self.deferred_chord_chart_preview = None;
        self.chord_chart_preview_command_id = None;
        self.chord_chart.set_preview_sounding(false);
    }

    /// その時刻に preview がまだ鳴っているか。
    ///
    /// command を渡した時刻から推測せず、sender が準備と timeline 送信を終えて公開した
    /// 実演奏区間だけを見る。別 command に置き換わった区間は一致しないので鳴っていない。
    pub(in crate::tui) fn chord_chart_preview_sounding(&self, now: Instant) -> bool {
        let playback = self
            .mml_overlay_sender
            .as_ref()
            .and_then(|sender| sender.status().line_playback())
            .map(|playback| (playback.command_id(), playback.is_sounding_at(now)));
        preview_command_sounding(self.chord_chart_preview_command_id, playback)
    }
    /// 要求 1 つを「送る内容」へ変換する。**曲の prefix をどこから取るかはここだけ**。
    ///
    /// 送る前に値として取り出せる形にしてあるのは、何を鳴らそうとしたかを
    /// テストで読むため（音そのものは機械で判定できないが、送った 1 行は読める）。
    #[cfg(test)]
    pub(in crate::tui) fn chord_chart_preview(
        &self,
        request: &PreviewRequest,
    ) -> ChordChartPreview {
        let bass_patch = self.resolve_chord_chart_bass_patch();
        self.chord_chart_preview_with_bass_patch(request, bass_patch)
    }

    fn chord_chart_preview_with_bass_patch(
        &self,
        request: &PreviewRequest,
        bass_patch: bass_patch::BassPatchResolution,
    ) -> ChordChartPreview {
        preview::build(
            &self.chord_chart.song.prefix,
            self.chord_chart_patch.clone(),
            self.chord_chart.bass_enabled(),
            bass_patch,
            request,
        )
    }

    /// 保存済み値を優先し、無ければ共有 catalog の Bass role 先頭を同期的に読む。
    /// catalog の読み込み完了を待たず、その時点の状態だけを返す。
    pub(in crate::tui) fn resolve_chord_chart_bass_patch(&self) -> bass_patch::BassPatchResolution {
        bass_patch::resolve(
            self.chord_chart_bass_patch.as_deref(),
            &self.patch_load_state.lock().unwrap(),
        )
    }

    /// 画面へ入るときに 1 度だけ呼ぶ。保存ファイルが読めなかったときの自動抽選
    /// （`g` 1 回ぶん）はここで走る。
    ///
    /// **起動時ではなくここで引く理由**: カタログは遅延取得で、キャッシュがまだ無い
    /// 初回は取得完了まで待つ（上限 20 秒）。`TuiApp::new` で引くと、chord chart を
    /// 開かない人まで起動をその時間止められる。
    ///
    /// 抽選できた曲はその場で保存する。保存しないと、次の起動でまた別の進行が出る。
    /// 入った直後の preview 要求（3.2 の 4 つめのきっかけ）もここで回収する。
    pub(in crate::tui) fn enter_chord_chart(&mut self) {
        if self.chord_chart.enter() == ChordChartAction::SongChanged {
            self.save_chord_chart();
        }
        // 抽選が走ったかどうかに関わらず切り直す。**画面を離れている間に曲が
        // 変わっている**ことがある（起動直後の load、他画面から `song` を触る経路）。
        self.refresh_chord_chart_chord_ranges();
        self.drain_chord_chart_preview();
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

/// 曲の全 section について「行内のどこからどこまでが 1 つの chord か」を切る。
///
/// **切り方は chord 1 つを鳴らすときと同じ関数**（`chord_source_ranges`）。
/// 画面が数を数えるのもこの写しの長さからで、`ChordProgression::chord_count()` は
/// 使わない。あれはハイフンだけを見る別物で、ユーザーが `i` で打った任意の degrees
/// では食い違いうる。
///
/// 読めない degrees は 0 件になる。ここでは 1 件へ丸めずそのまま渡し、
/// 「1 個として扱う」判断は画面側（`cursor_chord_count`）へ寄せる。
pub(in crate::tui) fn chord_ranges(song: &Song) -> Vec<(SectionId, Vec<std::ops::Range<usize>>)> {
    song.sections
        .iter()
        .map(|section| {
            (
                section.id,
                cmrt_chord::chord_source_ranges(&section.degrees),
            )
        })
        .collect()
}
