use super::*;

use cmrt_tui_core::buffer_test::{find_text_ignoring_spaces, help_overlay_bounds};

/// 4.5 のキーが全部読めるか。「押せるキーが分からない画面」にしないための固定。
#[test]
fn the_help_overlay_lists_every_key() {
    let screen = ChordChartScreen {
        help_open: true,
        ..ChordChartScreen::default()
    };

    let buffer = render(&screen);
    let rendered = buffer_to_string(&buffer).replace(' ', "");

    // 他画面と同じ目印。共有ヘルパが枠を見つけられることまで見る。
    find_text_ignoring_spaces(&buffer, "ヘルプ(Keybinds)");
    let (left, top, right, bottom) = help_overlay_bounds(&buffer);
    assert!(left < right && top < bottom);

    for key in [
        "h/l",
        "j/k",
        "PgUp/PgDn",
        "Alt+↑/↓",
        "Ctrl+G",
        "?/Esc",
        "g",
        "r",
        "i",
        "n",
        "dd",
        "1..9",
        "Shift+P",
        "Space",
    ] {
        assert!(rendered.contains(key), "key {key:?} is absent: {rendered}");
    }
    // 廃止したキーの説明が残っていると、押しても効かないキーを教える画面になる。
    for retired in ["[/]", "</>", "+/-", "J/K", "小節数を1..8"] {
        assert!(
            !rendered.contains(retired),
            "retired key {retired:?} is still listed: {rendered}"
        );
    }
    // `b` は 1 文字なので単体では他の行にも当たる。説明ごと照合する。
    assert!(rendered.contains("bKey/BPMを入力"), "{rendered}");
    // 書式の詳細はヘルプに書かない（`[user]` 指示。覚えるものを増やさない）。
    // **枠の中だけを読む**。画面全体だと裏のヘッダ（`Key=C BPM120`）を拾って必ず落ちる。
    let inside = overlay_text(&buffer, left, top, right, bottom);
    for detail in ["Key=", "BPM120", "TEMPO"] {
        assert!(
            !inside.contains(detail),
            "書式の詳細 {detail:?} がヘルプに残っている: {inside}"
        );
    }
    // pane ごとに意味が変わるキー（dd）があるので、見出しで区切る。
    assert!(rendered.contains("Sectionspane"), "{rendered}");
    assert!(rendered.contains("Arrangementpane"), "{rendered}");
}

/// ヘルプが教えるキーの**集合そのもの**を固定する。
///
/// `the_help_overlay_lists_every_key` の「廃止キーが残っていないこと」は、
/// `[/]` のような綴りを名指しで拒むので、**1 文字の廃止キー（`a` `e` `x` `d`）が
/// 戻ってきても素通しする**。ここは逆に「これ以外のキーは 1 つも教えない」を
/// 言うので、増えても減っても落ちる。増やすときは 7 章の承認が要る
/// （AI が勝手にキーを足したのが、そもそもこの削減の発端）。
#[test]
fn the_help_teaches_exactly_the_keys_that_survived_the_reduction() {
    let mut keys = crate::ui::help::HELP_ROWS
        .iter()
        .filter_map(|row| key_column(row))
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();

    assert_eq!(
        keys,
        [
            "1..9",
            "? / Esc",
            "Alt+↑/↓",
            "Ctrl+G",
            "PgUp/PgDn",
            "Shift+P",
            "Space",
            "b",
            "dd",
            "g",
            "h / l",
            "i",
            "j/k ↑↓",
            "n",
            "q",
            "r",
        ]
    );
}

/// 下段要約が挙げるキーも同じ範囲に収まっているか。
///
/// 要約は overlay より先に目に入るので、ここへ廃止キーが残ると
/// 「押しても効かないキー」を最初に教える画面になる。
#[test]
fn the_bottom_line_names_no_key_outside_that_set() {
    let keys = crate::ui::help::KEYBIND_TEXT
        .split_whitespace()
        .filter_map(|item| item.split(':').next())
        .collect::<Vec<_>>();

    assert_eq!(keys, ["q", "?"]);
}

/// ヘルプ 1 行の「キーの欄」。`キー` + 2 桁以上の空白 + `説明` という形なので、
/// 空白 2 つで切る。見出し行（`── Sections pane ──`）と空行は `None`。
fn key_column(row: &str) -> Option<&str> {
    let key = row.split("  ").next()?.trim();
    if key.is_empty() || key.starts_with('─') {
        return None;
    }
    Some(key)
}

/// help overlay の枠の内側だけを、空白を落として 1 本の文字列で返す。
///
/// 「ヘルプに書いていないこと」を見る assert は、画面全体を読むと裏の画面の文字を
/// 拾ってしまう（ヘッダの `Key=C BPM120` で実際に落ちた）。
fn overlay_text(buffer: &Buffer, left: u16, top: u16, right: u16, bottom: u16) -> String {
    (top..=bottom)
        .flat_map(|y| {
            (left..=right).map(move |x| buffer.cell((x, y)).unwrap().symbol().to_string())
        })
        .collect::<String>()
        .replace(' ', "")
}

#[test]
fn the_help_overlay_is_hidden_until_it_is_opened() {
    let buffer = render(&ChordChartScreen::default());
    let rendered = buffer_to_string(&buffer);

    assert!(!rendered.contains("ヘルプ"), "{rendered}");
}

/// 下段は `q` と `?` だけ。`?` があることが「ヘルプの開き方」の唯一の手掛かり。
#[test]
fn the_bottom_line_summarises_the_keys() {
    let buffer = render(&ChordChartScreen::default());
    let bottom = row_text(&buffer, buffer.area.height - 2);

    // 全角はセル 2 つを占めるので、内容の照合は `squeeze` を通す。
    assert!(squeeze(&bottom).contains("q:終了"), "{bottom:?}");
    // 末尾まで 80 桁に収まっていないと、ヘルプへの入口が切れて消える。
    assert!(bottom.contains("?:help"), "{bottom:?}");
    // 他のキーは 1 つも載せない（`[user]` 指示。全量は `?` 側）。
    for gone in [
        "hl:pane",
        "g:抽選",
        "r:引直し",
        "i:進行",
        "n:名前",
        "dd:",
        "Alt+",
    ] {
        assert!(!squeeze(&bottom).contains(gone), "{bottom:?}");
    }
}

/// 何もできなかった理由がある間は、キー要約より理由を優先する。
#[test]
fn an_error_replaces_the_key_summary() {
    let screen = ChordChartScreen {
        error: Some("コード進行データがありません".to_string()),
        ..ChordChartScreen::default()
    };

    let buffer = render(&screen);
    let bottom = row_text(&buffer, buffer.area.height - 2);

    assert!(
        squeeze(&bottom).contains("コード進行データがありません"),
        "{bottom:?}"
    );
    assert!(!bottom.contains("?:help"), "{bottom:?}");
}

/// カタログがまだ無いときの `g` は、無反応ではなく理由が画面に出る。
/// （`error` を立てるだけで下段へ届いていない、という取りこぼしを塞ぐ）
#[test]
fn pressing_g_without_a_catalog_puts_the_reason_on_the_bottom_line() {
    let mut screen = ChordChartScreen::default();

    screen.handle_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('g'),
        crossterm::event::KeyModifiers::NONE,
    ));

    let buffer = render(&screen);
    let bottom = row_text(&buffer, buffer.area.height - 2);
    assert!(
        squeeze(&bottom).contains("コード進行データがありません"),
        "{bottom:?}"
    );
}

/// 下段要約が 80 桁端末に収まるか。**桁数そのもの**を固定する。
///
/// `?:help` の有無だけを見ていると、キーを 1 つ足した瞬間に末尾が黙って切れて
/// 「ヘルプの開き方」が画面から消える（前資料 Stage 3 で実際に起きた）。
/// 上限は 80 桁のときの下段の実幅から取るので、レイアウトを変えても追従する。
#[test]
fn the_bottom_line_fits_in_an_eighty_column_terminal() {
    let available = layout_for(Rect::new(0, 0, TEST_WIDTH, TEST_HEIGHT))
        .status
        .width;
    assert_eq!(available, 78, "80 桁端末の下段は枠の内側 78 桁");

    // 全角を 2 桁として数えた実幅。ratatui が切り詰めるのと同じ勘定。
    let width = status_line(&ChordChartScreen::default()).width();

    assert!(
        width <= available as usize,
        "下段要約が {width} 桁あり、{available} 桁に収まらない"
    );
}

/// ヘルプ overlay が 80x24 に収まり、いちばん長い行も切れずに出るか。
#[test]
fn the_help_overlay_fits_inside_an_eighty_column_terminal() {
    let screen = ChordChartScreen {
        help_open: true,
        ..ChordChartScreen::default()
    };

    let buffer = render(&screen);
    let (left, top, right, bottom) = help_overlay_bounds(&buffer);

    assert!(right < TEST_WIDTH, "枠の右端が画面の外: right={right}");
    assert!(bottom < TEST_HEIGHT, "枠の下端が画面の外: bottom={bottom}");

    // いちばん長い説明行。切り詰められていれば末尾の `)` が落ちる。
    // **綴りを直書きせず、実際にいちばん長い行を計算して選ぶ**（行を足したときに
    // 「もう最長ではない行」を見張り続けて素通しするのを防ぐ）。
    let longest = crate::ui::help::HELP_ROWS
        .iter()
        .max_by_key(|row| ratatui::text::Span::raw(**row).width())
        .unwrap();
    let inside = overlay_text(&buffer, left, top, right, bottom);
    assert!(
        inside.contains(&squeeze(longest)),
        "いちばん長い行 {longest:?} が切れている: {inside}"
    );
}
