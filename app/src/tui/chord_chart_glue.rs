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

use cmrt_mml_overlay::line_play::{line_events, LineProgram, LineStatus};

use crate::tui::chord_chart::{ChordChartAction, PreviewRequest, SectionId, Song};
use crate::tui::TuiApp;

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
            ChordChartAction::Continue | ChordChartAction::Quit => {}
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
        crate::logging::global_log_sink(&preview_request_log_line(&request));
        let preview = self.chord_chart_preview(&request);
        crate::logging::global_log_sink(&preview_play_log_line(&preview));
        // 読めなかった理由は、鳴らせるかどうかに関係なく画面へ出す
        // （play server が上がっていないときも、理由は理由として見せる）。
        if let LineStatus::Error(reason) = &preview.status {
            self.chord_chart.error = Some(format!("鳴らせません: {reason}"));
        }
        // play server が上がっていなければ sender が無い。落とさず、何もしない。
        self.chord_chart_preview_command_id = self.mml_overlay_sender.as_ref().map(|sender| {
            if preview.program.is_silent() {
                sender.stop()
            } else {
                sender.play_line(preview.patch.as_deref(), preview.program)
            }
        });
        self.refresh_chord_chart_preview_sounding();
    }

    /// 「preview がまだ鳴っているか」の答えを画面へ書き戻す。
    ///
    /// 画面 crate は音を持たないので自分では知り得ない（`set_preview_sounding` の
    /// doc を参照）。glue が preview を投げた直後と、キーを渡す直前に更新する。
    fn refresh_chord_chart_preview_sounding(&mut self) {
        let sounding = self.chord_chart_preview_sounding(Instant::now());
        self.chord_chart.set_preview_sounding(sounding);
    }

    /// 音源を他へ明け渡したので、「鳴っている」という記録を捨てる。
    ///
    /// **止めるコマンドはここからは出さない。** 明け渡した先が音源ごと止めるため
    /// （MML オーバーレイを開くと `sender.prepare()` が走り、その中の `voice.stop` が
    /// 走っている timeline を落とす。`mml-overlay/src/sender/tests.rs` の
    /// `preparing_an_already_ready_patch_stops_the_previous_line` が固定している）。
    /// 記録だけが残ると、戻ってきたときの `Space` が「止める」に化けて空打ちになる。
    pub(in crate::tui) fn forget_chord_chart_preview(&mut self) {
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
    pub(in crate::tui) fn chord_chart_preview(
        &self,
        request: &PreviewRequest,
    ) -> ChordChartPreview {
        chord_chart_preview(
            &self.chord_chart.song.prefix,
            self.chord_chart_patch.clone(),
            request,
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

/// preview 要求 1 つを、ログ 1 行にする。
///
/// **`global_log_sink` はテストでは no-op** なので、組み立てだけを名前のある関数へ
/// 出しておく。こうしないと「何を鳴らそうとしたか」を機械で確かめる手段が無くなる。
pub(in crate::tui) fn preview_request_log_line(request: &PreviewRequest) -> String {
    format!(
        "chord-chart: event=preview-request name=\"{}\" degrees=\"{}\" chord={}",
        request.name,
        request.degrees,
        match request.chord_index {
            Some(index) => index.to_string(),
            None => "all".to_string(),
        }
    )
}

/// 1 回ぶんの preview を、実際に送る形まで組み立てたもの。
pub(in crate::tui) struct ChordChartPreview {
    /// 演奏側へ渡した 1 行。無音要求なら空。
    pub line: String,
    /// その 1 行がどう読まれたか。`Error` なら画面下段に理由を出す。
    pub status: LineStatus,
    /// sender へ渡すもの。**1 回鳴って終わる**（`repeat` しない）。
    pub program: LineProgram,
    /// この通常 preview に使う Chord Chart canonical patch。
    pub patch: Option<String>,
    /// 行全体ではなく chord 1 つに絞ったなら `(0 始まりの番号, 行の chord 総数)`。
    ///
    /// **行全体を鳴らしたときは `None`**。要求が番号を指していても、読めない
    /// degrees や範囲外で行全体へ倒れたときは `None` になる（ログを見るだけで
    /// 「1 つに絞れたのか、倒れたのか」が分かるように、要求の番号をそのまま
    /// 写さない）。
    pub chord: Option<(usize, usize)>,
}

/// preview 要求と曲の prefix から、送る内容を組み立てる。
///
/// **この関数だけが文字列を組み立てる**（`chord-chart` crate は文字列を解釈しないし、
/// 組み立てもしない。ADR 0020）。
fn chord_chart_preview(
    prefix: &str,
    patch: Option<String>,
    request: &PreviewRequest,
) -> ChordChartPreview {
    if request.is_silent() {
        return ChordChartPreview {
            line: String::new(),
            status: LineStatus::Idle,
            program: LineProgram::silent(),
            patch,
            chord: None,
        };
    }
    let chord = request
        .chord_index
        .and_then(|index| chord_at(&request.degrees, index));
    let degrees = match &chord {
        Some((degrees, _)) => degrees.as_str(),
        None => request.degrees.as_str(),
    };
    let line = preview_line(prefix, degrees);
    let (status, performance) = line_events(&line);
    ChordChartPreview {
        line,
        status,
        program: LineProgram::once(performance),
        patch,
        chord: chord.map(|(_, total)| (request.chord_index.unwrap_or(0), total)),
    }
}

/// sender の実演奏区間が、Chord Chart が最後に送った command と一致して鳴っているか。
///
/// `None` は patch load / timeline 送信中、失敗、停止をまとめて silent とする。
/// command id が違えば別 owner または後続 preview に置き換わっているため silent とする。
pub(in crate::tui) fn preview_command_sounding(
    expected_command_id: Option<u64>,
    playback: Option<(u64, bool)>,
) -> bool {
    matches!(
        (expected_command_id, playback),
        (Some(expected), Some((actual, true))) if expected == actual
    )
}

/// 行の degrees から `index` 番目（0 始まり）の chord だけを切り出し、
/// 切り出した綴りと**その行の chord 総数**を返す。
///
/// **切るのは `cmrt-chord`**（`chord_source_ranges` が chord2mml の CST から
/// 元の文字列上の範囲を返す）。この画面のためのパーサは app 側にも 1 行も書かない。
///
/// `None`（＝行全体へ倒す）になるのは 3 つ:
///
/// - chord2mml が読めない degrees（範囲が 0 件）。「読めない行は chord 1 個」として
///   扱い、行全体をそのまま鳴らす
/// - 番号が範囲外（画面の写しが古いときに起きうる）
/// - 範囲が文字境界で切れない（起きないはずだが、`get` で panic させない）
fn chord_at(degrees: &str, index: usize) -> Option<(String, usize)> {
    let ranges = cmrt_chord::chord_source_ranges(degrees);
    let range = ranges.get(index)?.clone();
    let chord = degrees.get(range)?;
    Some((chord.to_string(), ranges.len()))
}

/// 鳴らす 1 行 `"<Key トークン> <degrees>"`。Key トークンが無ければ degrees だけ。
pub(in crate::tui) fn preview_line(prefix: &str, degrees: &str) -> String {
    match key_token(prefix) {
        Some(key) => format!("{key} {degrees}"),
        None => degrees.to_string(),
    }
}

/// prefix（`"Key=C BPM120"`）から Key トークンを 1 つだけ取り出す。
///
/// **BPM / TEMPO は捨てる。**曲の prefix はそのまま持つ文字列で、
/// 何が書いてあるかは保証されていないため、渡すものをここで絞る。
///
/// 判定は「`key` で始まるトークン（大文字小文字を問わない）の最初の 1 つ」。
/// chord2mml は `Key=A` / `Key:A` / `Key A` / `KeyA` を受けるが、`Key A` のように
/// 空白で割れた書き方はトークンが 2 つになるので `Key` だけが渡る。
pub(in crate::tui) fn key_token(prefix: &str) -> Option<&str> {
    prefix
        .split_whitespace()
        .find(|token| token.to_ascii_lowercase().starts_with("key"))
}

/// 実際に何を送ったかのログ 1 行。要求のログ（[`preview_request_log_line`]）とは別に、
/// **組み立てた 1 行とその読まれ方**を残す。鳴らなかったときに、要求が立たなかったのか
/// 文字列が読めなかったのかをログだけで切り分けるため。
pub(in crate::tui) fn preview_play_log_line(preview: &ChordChartPreview) -> String {
    let result = match &preview.status {
        LineStatus::Idle => "result=silent".to_string(),
        LineStatus::Played {
            from_chord,
            note_count,
        } => format!("result=played from_chord={from_chord} notes={note_count}"),
        LineStatus::Error(error) => format!("result=error detail=\"{error}\""),
    };
    let chord = match preview.chord {
        Some((index, total)) => format!("{index}/{total}"),
        None => "all".to_string(),
    };
    format!(
        "chord-chart: event=preview-play line=\"{}\" chord={chord} {result}",
        preview.line
    )
}
