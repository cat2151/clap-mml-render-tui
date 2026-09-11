//! 行の中の **chord 1 つだけ**を鳴らす経路（glue が degrees を切り出すところ）。
//!
//! 行 1 本ぶんの preview は [`super::chord_chart_preview`] が見る。ここで見るのは
//! 「番号を指したときに**送る 1 行**が本当に chord 1 つになるか」と、切り出せない
//! ときに行全体へ倒れるか。切るのは `cmrt-chord`（`chord_source_ranges`）で、
//! 画面 crate は degrees を解釈しない（ADR 0020）。

use super::chord_chart_preview::{app_on_the_chord_chart, chord_onsets};
use crate::tui::chord_chart_glue::{preview_line, preview_play_log_line, preview_request_log_line};
use cmrt_chord_chart::PreviewRequest;
use cmrt_mml_overlay::line_play::LineStatus;

/// 行内の 1 つを指した要求は、**その chord 1 つだけ**の行になる。
///
/// 送る 1 行そのものを読む（`the_line_the_glue_sends_really_plays_as_a_chord_progression`
/// と同じ流儀）。行全体との差は note 数でも見る。12 音 → 3 音。
#[test]
fn a_chord_index_narrows_the_line_to_that_single_chord() {
    let app = app_on_the_chord_chart();

    let preview = app.chord_chart_preview(&PreviewRequest {
        name: "A".to_string(),
        degrees: "I-V-VIm-IV".to_string(),
        chord_index: Some(1),
    });

    assert_eq!(
        preview.line, "Key=C V",
        "Key トークンは付いたまま、degrees だけが 2 番目の chord に絞られること"
    );
    assert_eq!(
        preview.status,
        LineStatus::Played {
            from_chord: true,
            note_count: 3
        },
        "行全体の 12 音より少ないこと"
    );
    assert_eq!(preview.chord, Some((1, 4)), "何番目 / 全何個を持ち帰ること");
    let events = preview.program.events();
    assert_eq!(events.len(), 6, "note on 3 と note off 3");
    assert_eq!(
        events.last().unwrap().message[0] & 0xf0,
        0x80,
        "最後のイベントは note off であること（鳴らしっぱなしにしない）"
    );
    // 本当に V（G 和音）が鳴ること。行全体の 2 番目と同じ音高。
    assert_eq!(chord_onsets(&preview), vec![(0.0, vec![67, 71, 74])]);
}

/// 全部の番号を順に指すと、行全体を 1 和音ずつ辿ったのと同じ音になる。
///
/// 1 つずつの切り出しが**位置ごとにずれていない**ことを、行全体の実測と
/// 突き合わせて確かめる（件数だけ合っていても位置はずれうる）。
#[test]
fn every_index_sounds_the_same_chord_as_that_position_of_the_whole_line() {
    let app = app_on_the_chord_chart();
    let whole = app.chord_chart_preview(&PreviewRequest {
        name: "A".to_string(),
        degrees: "I-V-VIm-IV".to_string(),
        chord_index: None,
    });

    let one_by_one: Vec<Vec<u8>> = (0..4)
        .map(|index| {
            let preview = app.chord_chart_preview(&PreviewRequest {
                name: "A".to_string(),
                degrees: "I-V-VIm-IV".to_string(),
                chord_index: Some(index),
            });
            chord_onsets(&preview)
                .first()
                .map(|(_, notes)| notes.clone())
                .unwrap_or_default()
        })
        .collect();

    let from_whole: Vec<Vec<u8>> = chord_onsets(&whole)
        .into_iter()
        .map(|(_, notes)| notes)
        .collect();
    assert_eq!(one_by_one, from_whole);
}

/// 行全体（`None`）の要求は、chord 1 つを足す前と**1 文字も変わらない**行を送る。
#[test]
fn a_whole_line_request_sends_the_very_same_line_as_before() {
    let app = app_on_the_chord_chart();

    let preview = app.chord_chart_preview(&PreviewRequest {
        name: "A".to_string(),
        degrees: "I-V-VIm-IV".to_string(),
        chord_index: None,
    });

    assert_eq!(preview.line, "Key=C I-V-VIm-IV");
    assert_eq!(
        preview.line,
        preview_line(&app.chord_chart.song.prefix, "I-V-VIm-IV"),
        "行全体の組み立ては `preview_line` そのままであること"
    );
    assert_eq!(preview.chord, None, "絞っていないので番号を持たないこと");
}

/// 読めない degrees で番号を指しても、**行全体のときと同じ結果**になる。
/// 範囲外の番号（画面の写しが古いとき）も同じく行全体へ倒れる。
#[test]
fn an_index_that_cannot_be_cut_falls_back_to_the_whole_line() {
    let mut app = app_on_the_chord_chart();
    app.chord_chart.song.prefix = String::new();

    for (degrees, index) in [
        ("zzz", 0),
        ("zzz", 3),
        ("I-V-VIm-IV", 4),
        ("I-V-VIm-IV", 99),
    ] {
        let narrowed = app.chord_chart_preview(&PreviewRequest {
            name: "A".to_string(),
            degrees: degrees.to_string(),
            chord_index: Some(index),
        });
        let whole = app.chord_chart_preview(&PreviewRequest {
            name: "A".to_string(),
            degrees: degrees.to_string(),
            chord_index: None,
        });

        assert_eq!(
            narrowed.line, whole.line,
            "{degrees:?} の {index} 番は行全体へ倒れること"
        );
        assert_eq!(narrowed.status, whole.status);
        assert_eq!(
            narrowed.chord, None,
            "倒れたことがログで分かるよう、番号は持ち帰らないこと"
        );
    }
}

/// 全角を含む綴りでも、バイト境界で割らずに切り出せる（割ると panic する位置がある）。
#[test]
fn a_full_width_chord_name_is_cut_at_a_character_boundary() {
    let mut app = app_on_the_chord_chart();
    app.chord_chart.song.prefix = String::new();

    let preview = app.chord_chart_preview(&PreviewRequest {
        name: "A".to_string(),
        degrees: "C♯m7-F♯7-BM7".to_string(),
        chord_index: Some(1),
    });

    assert_eq!(preview.line, "F♯7");
    assert_eq!(preview.chord, Some((1, 3)));
    assert!(!preview.program.is_silent(), "切り出した綴りが鳴ること");
}

/// 「行全体を鳴らしたのか chord 1 つだったのか」をログだけで切り分けられること。
///
/// **番号は 0 始まり**（画面が持つ chord カーソルと同じ値）。総数は要求ではなく
/// 実際に切れた数なので、要求のログには出せない（出すには degrees をもう一度
/// 読み直すことになる）。総数が出るのは送ったほうのログ。
#[test]
fn the_log_lines_tell_a_single_chord_from_the_whole_line() {
    let mut app = app_on_the_chord_chart();

    let request = PreviewRequest {
        name: "A".to_string(),
        degrees: "I-V-VIm-IV".to_string(),
        chord_index: Some(1),
    };
    assert_eq!(
        preview_request_log_line(&request),
        concat!(
            "chord-chart: event=preview-request name=\"A\"",
            " degrees=\"I-V-VIm-IV\" chord=1"
        )
    );
    assert_eq!(
        preview_play_log_line(&app.chord_chart_preview(&request)),
        concat!(
            "chord-chart: event=preview-play line=\"Key=C V\"",
            " chord=1/4 result=played from_chord=true notes=3"
        ),
        "何番目 / 全何個が 1 行に出ること"
    );

    // 読めない degrees は番号を指しても行全体。ログも `all` になる。
    app.chord_chart.song.prefix = String::new();
    let unreadable = PreviewRequest {
        name: "B".to_string(),
        degrees: "zzz".to_string(),
        chord_index: Some(0),
    };
    assert_eq!(
        preview_play_log_line(&app.chord_chart_preview(&unreadable)),
        concat!(
            "chord-chart: event=preview-play line=\"zzz\" chord=all",
            " result=error detail=\"MMLに発音ノートがありません\""
        ),
        "切り出せずに倒れたことがログで分かること"
    );
}
