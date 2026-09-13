//! preview 要求が app 側 glue まで届き、そこで回収されること。
//!
//! 要求が立つ / 立たないの境目は `cmrt-chord-chart` の `screen::preview::tests` が見る。
//! ここで見るのは「glue が繋がっているか」「ログ 1 行の綴り」と、
//! **実際に演奏側へ送った 1 行**（音そのものは機械で判定できないが、送った文字列と
//! それがどう読まれたかは読める）。

use super::*;
use crate::screen_switch::PrimaryScreen;
use crate::tui::chord_chart_glue::{preview_line, preview_play_log_line, preview_request_log_line};
use cmrt_chord_chart::PreviewRequest;
use cmrt_mml_overlay::line_play::LineStatus;

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// section 2 つ・arrangement 1 行の曲を入れた chord chart 画面。
/// トグルのテスト（`chord_chart_toggle`）も同じ曲を使う。
pub(super) fn app_on_the_chord_chart<'a>() -> TuiApp<'a> {
    let mut app = TuiApp::new_for_test(test_config());
    let mut song = cmrt_chord_chart::Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "IIm-V-I-VIm");
    song.arrangement = vec![a];
    app.chord_chart = cmrt_chord_chart::ChordChartScreen::new(song);
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app
}

/// カーソルを動かすと要求が立ち、**その場で glue が回収する**。
///
/// 「消費された」だけでは「そもそも立たなかった」と区別できないので、同じ状態の
/// 複製へ同じキーを打って**立つこと**を先に確かめる（対照実験）。
#[test]
fn a_cursor_move_raises_a_request_and_the_glue_consumes_it() {
    let mut app = app_on_the_chord_chart();
    // 入った直後の 1 回ぶんは glue が回収済み。
    assert_eq!(app.chord_chart.take_preview(), None);

    let mut unglued = app.chord_chart.clone();
    unglued.handle_key_event(plain(KeyCode::Char('j')));
    assert_eq!(
        unglued.take_preview(),
        Some(PreviewRequest {
            name: "B".to_string(),
            degrees: "IIm-V-I-VIm".to_string(),
            chord_index: None,
        }),
        "画面単体なら要求が立つこと（対照）"
    );

    app.handle_chord_chart_key_event(plain(KeyCode::Char('j')));

    assert_eq!(
        app.chord_chart.take_preview(),
        None,
        "glue を通ったら要求は残らないこと"
    );
}

/// 画面へ入った直後の 1 回も glue が回収する（`switch_to_primary_screen` 経由）。
#[test]
fn entering_the_screen_consumes_the_first_request() {
    let mut app = TuiApp::new_for_test(test_config());
    let mut song = cmrt_chord_chart::Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    app.chord_chart = cmrt_chord_chart::ChordChartScreen::new(song);

    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert_eq!(app.chord_chart.take_preview(), None);
}

/// ログ 1 行の綴り。`global_log_sink` はテストでは no-op なので、組み立てを直接見る。
#[test]
fn the_log_line_names_the_section_and_the_degrees() {
    let request = PreviewRequest {
        name: "A".to_string(),
        degrees: "I-V-VIm-IV".to_string(),
        chord_index: None,
    };

    assert_eq!(
        preview_request_log_line(&request),
        "chord-chart: event=preview-request name=\"A\" degrees=\"I-V-VIm-IV\" chord=all"
    );
}

/// 「止めるだけ」の要求も 1 行出す。何も出さないと、鳴らなかったのがバグなのか
/// 無音要求だったのかがログから分からない。
#[test]
fn a_silent_request_is_logged_with_empty_fields() {
    assert_eq!(
        preview_request_log_line(&PreviewRequest::silent()),
        "chord-chart: event=preview-request name=\"\" degrees=\"\" chord=all"
    );
}

/// 3.5 の「Key トークンを 1 つだけ渡す」。prefix はそのまま持つ文字列なので、
/// 何が書いてあっても渡すものはここで絞る。
#[test]
fn only_the_first_key_token_of_the_prefix_reaches_the_player() {
    assert_eq!(preview_line("Key=C BPM120", "I-V"), "Key=C I-V");
    assert_eq!(preview_line("key:a", "I-V"), "key:a I-V");
    assert_eq!(preview_line("BPM120 Key=D", "I-V"), "Key=D I-V");
    assert_eq!(
        preview_line("BPM120", "I-V"),
        "I-V",
        "Key が無ければ degrees だけ"
    );
    assert_eq!(preview_line("", "I-V"), "I-V");
}

/// 組み立てた 1 行が**本当にコード進行として鳴る**こと。
///
/// ここが `line_events` の実測。「送る文字列は組めたが演奏側が読めなかった」を
/// 見逃さないため、note 数まで見る（`I-V-VIm-IV` の 4 和音 × 3 音 = 12）。
#[test]
fn the_line_the_glue_sends_really_plays_as_a_chord_progression() {
    let app = app_on_the_chord_chart();

    let preview = app.chord_chart_preview(&PreviewRequest {
        name: "A".to_string(),
        degrees: "I-V-VIm-IV".to_string(),
        chord_index: None,
    });

    assert_eq!(
        preview.line, "Key=C I-V-VIm-IV",
        "prefix の Key だけが渡ること（既定 prefix は \"Key=C BPM120\"）"
    );
    assert_eq!(
        preview.status,
        LineStatus::Played {
            from_chord: true,
            note_count: 12
        }
    );
    assert!(!preview.program.is_silent());
    assert!(!preview.program.repeat, "preview は 1 回鳴って終わる");
    // note off まで積まれていること。積まれていないと、鳴らしっぱなしのまま
    // 画面を離れられてしまう（明け渡しのとき止める必要が出る）。
    let events = preview.program.events();
    assert_eq!(events.len(), 24, "note on 12 と note off 12");
    assert_eq!(
        events
            .iter()
            .filter(|e| e.message[0] & 0xf0 == 0x80)
            .count(),
        12,
        "全部の note on に note off が付くこと"
    );
    assert_eq!(
        events.last().unwrap().message[0] & 0xf0,
        0x80,
        "最後のイベントは note off であること"
    );
}

/// 無音要求（行が無い / 参照切れ / degrees が空）は「止めるだけ」を送る。
/// 何も送らないと、空の行へ降りても前の section が鳴り続ける。
#[test]
fn a_silent_request_sends_a_program_that_only_stops_the_previous_sound() {
    let app = app_on_the_chord_chart();

    let preview = app.chord_chart_preview(&PreviewRequest::silent());

    assert_eq!(preview.line, "");
    assert_eq!(preview.status, LineStatus::Idle);
    assert!(preview.program.is_silent());
}

/// 読めない degrees は無音になり、理由が画面下段へ出る。
///
/// **prefix から Key を外した曲で見る。** Key トークンが付いていると、演奏側は
/// 読めなかった行を MML として読み直し、`Key=C` の `e` と `c` を音として鳴らして
/// しまう。エラーになるのは Key の無い行だけ。
#[test]
fn an_unreadable_progression_falls_silent_and_says_why() {
    let mut app = TuiApp::new_for_test(test_config());
    let mut song = cmrt_chord_chart::Song::empty();
    song.prefix = String::new();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "zzz");
    app.chord_chart = cmrt_chord_chart::ChordChartScreen::new(song);
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('j')));

    assert_eq!(
        app.chord_chart.error.as_deref(),
        Some("鳴らせません: MMLに発音ノートがありません")
    );
    let preview = app.chord_chart_preview(&PreviewRequest {
        name: "B".to_string(),
        degrees: "zzz".to_string(),
        chord_index: None,
    });
    assert!(preview.program.is_silent(), "読めない行は無音を送ること");
}

/// 読めなかったことを section の行に `!` として描かない（ADR 0020）。
/// 理由は下段 1 行だけに出す。
#[test]
fn an_unreadable_progression_puts_no_mark_on_the_section_row() {
    let mut app = TuiApp::new_for_test(test_config());
    let mut song = cmrt_chord_chart::Song::empty();
    song.prefix = String::new();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "zzz");
    app.chord_chart = cmrt_chord_chart::ChordChartScreen::new(song);
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.handle_chord_chart_key_event(plain(KeyCode::Char('j')));

    let rows = chord_chart_rows(&app);

    let pane_rows = sections_pane_rows(&rows);
    assert!(
        pane_rows.iter().any(|row| row.contains("zzz")),
        "読めない進行の行そのものは描かれること: {pane_rows:?}"
    );
    assert!(
        pane_rows.iter().all(|row| !row.contains('!')),
        "section の行に `!` 印を出さないこと: {pane_rows:?}"
    );
    // 下段 1 行は枠の 1 つ内側。全角はセル 2 つを使うので、空白を落として照合する。
    let status = &rows[rows.len() - 2];
    let squeezed: String = status.chars().filter(|ch| !ch.is_whitespace()).collect();
    assert!(
        squeezed.contains("鳴らせません:MMLに発音ノートがありません"),
        "理由は下段 1 行に出ること: {status:?}"
    );
}

/// play server が上がっていない（`mml_overlay_sender` が `None`）ときも落ちない。
/// preview は「鳴らせたら鳴らす」であって、鳴らせないことは異常ではない。
#[test]
fn a_missing_sender_is_not_an_error() {
    let mut app = app_on_the_chord_chart();
    assert!(
        app.mml_overlay_sender.is_none(),
        "テストの app は sender を持たない"
    );

    app.handle_chord_chart_key_event(plain(KeyCode::Char('j')));
    app.handle_chord_chart_key_event(plain(KeyCode::Tab));
    app.handle_chord_chart_key_event(plain(KeyCode::Char('k')));

    assert_eq!(app.chord_chart.error, None);
}

/// 行全体と chord 単体のどちらも Chord Chart の canonical patch を送信計画へ載せる。
/// 未選択なら `None` のまま realtime play server の既定音色へ倒す。
#[test]
fn every_normal_preview_uses_the_chord_chart_patch() {
    let mut app = app_on_the_chord_chart();
    let request = |chord_index| PreviewRequest {
        name: "A".to_string(),
        degrees: "I-V-VIm-IV".to_string(),
        chord_index,
    };

    assert_eq!(app.chord_chart_preview(&request(None)).patch, None);

    app.chord_chart_patch = Some("Keys/Stage Piano.fxp".to_string());
    assert_eq!(
        app.chord_chart_preview(&request(None)).patch.as_deref(),
        Some("Keys/Stage Piano.fxp")
    );
    assert_eq!(
        app.chord_chart_preview(&request(Some(2))).patch.as_deref(),
        Some("Keys/Stage Piano.fxp")
    );
}

/// 送った 1 行のログ。要求のログと合わせて、鳴らなかった理由をログだけで切り分ける。
#[test]
fn the_play_log_line_names_the_line_and_how_it_was_read() {
    let app = app_on_the_chord_chart();

    let played = app.chord_chart_preview(&PreviewRequest {
        name: "A".to_string(),
        degrees: "I-V-VIm-IV".to_string(),
        chord_index: None,
    });
    assert_eq!(
        preview_play_log_line(&played),
        concat!(
            "chord-chart: event=preview-play line=\"Key=C I-V-VIm-IV\"",
            " chord=all result=played from_chord=true notes=12"
        )
    );

    let silent = app.chord_chart_preview(&PreviewRequest::silent());
    assert_eq!(
        preview_play_log_line(&silent),
        "chord-chart: event=preview-play line=\"\" chord=all result=silent"
    );

    let mut app = app;
    app.chord_chart.song.prefix = String::new();
    let failed = app.chord_chart_preview(&PreviewRequest {
        name: "B".to_string(),
        degrees: "zzz".to_string(),
        chord_index: None,
    });
    assert_eq!(
        preview_play_log_line(&failed),
        concat!(
            "chord-chart: event=preview-play line=\"zzz\"",
            " chord=all result=error detail=\"MMLに発音ノートがありません\""
        )
    );
}

/// chord chart 画面を 80x24 で描いて、枠の内側の行だけを返す。
pub(super) fn chord_chart_rows(app: &TuiApp<'_>) -> Vec<String> {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|f| cmrt_chord_chart::ui::draw(&app.chord_chart, f))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect()
}

/// 左 pane（Sections）の枠の内側だけを返す。行番号で切ると、枠や下段まで巻き込む。
pub(super) fn sections_pane_rows(rows: &[String]) -> Vec<String> {
    rows.iter()
        .skip_while(|row| !row.contains("Sections"))
        .skip(1)
        .take_while(|row| !row.contains('└'))
        // `││  1 A  ...  ││ ... ││` の 2 つめの断片が左 pane の中身。
        .filter_map(|row| row.split("││").nth(1).map(str::to_string))
        .collect()
}

/// **何の和音が鳴るか**まで機械で見る。音色や快適さは耳の話だが、
/// 「調が効いているか」「進行の順に鳴るか」「1 和音がどれだけ伸びるか」は値で読める。
#[test]
fn the_preview_really_sounds_the_progression_in_the_key_of_the_prefix() {
    let mut app = app_on_the_chord_chart();
    let request = PreviewRequest {
        name: "A".to_string(),
        degrees: "I-V-VIm-IV".to_string(),
        chord_index: None,
    };

    let in_c = chord_onsets(&app.chord_chart_preview(&request));

    assert_eq!(
        in_c,
        vec![
            (0.0, vec![60, 64, 67]),
            (2.0, vec![67, 71, 74]),
            (4.0, vec![69, 72, 76]),
            (6.0, vec![65, 69, 72]),
        ]
    );

    // Key を書き換えると鳴る音が移る＝prefix の Key が本当に演奏へ届いている。
    app.chord_chart.song.prefix = "Key=D BPM120".to_string();
    let in_d = chord_onsets(&app.chord_chart_preview(&request));

    assert_eq!(
        in_d.first().map(|(_, notes)| notes.clone()),
        Some(vec![62, 66, 69]),
        "Key=D なら I は D 和音"
    );
    assert_eq!(in_d.len(), 4);
}

/// note on を時刻ごとにまとめて `(秒, 音高)` にする。和音は同じ時刻に固まる。
pub(super) fn chord_onsets(
    preview: &crate::tui::chord_chart_glue::ChordChartPreview,
) -> Vec<(f64, Vec<u8>)> {
    let mut onsets: Vec<(f64, Vec<u8>)> = Vec::new();
    for event in preview.program.events() {
        if event.message[0] & 0xf0 != 0x90 {
            continue;
        }
        match onsets.last_mut() {
            Some((seconds, notes)) if (*seconds - event.seconds).abs() < 1e-6 => {
                notes.push(event.message[1])
            }
            _ => onsets.push((event.seconds, vec![event.message[1]])),
        }
    }
    onsets
}
