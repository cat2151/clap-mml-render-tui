//! NOTE grid の GAIN 欄。サーバーの auto-trim が効いているかを行ごとに見せる。

use super::*;

fn connection_with_gains(gains: &[(usize, f32)]) -> GridConnectionStatus {
    let mut connection = GridConnectionStatus::default();
    for (instance_id, gain_db) in gains {
        connection.auto_gain_db[*instance_id] = *gain_db;
    }
    connection
}

/// GAIN 欄の左端。中央寄せで grid の左端が動くので、列は直書きせず layout から引く。
fn gain_column(screen: &GridSequencerScreen) -> usize {
    usize::from(test_layout(screen).gain_column())
}

/// 見出しと値が同じ桁に並ぶこと。値だけ出ていても何の数字か分からない。
#[test]
fn the_gain_column_shows_the_auto_trim_of_each_instance() {
    let screen = screen_with_first_row(60, &[0]);
    let connection = connection_with_gains(&[(0, 3.0), (1, -1.5)]);

    let rendered = render_with_connection(&screen, &connection);
    let lines = rendered.lines().collect::<Vec<_>>();
    let column = gain_column(&screen);

    assert_eq!(slice_chars(lines[FIRST_ROW_Y - 1], column, 5), " GAIN");
    assert_eq!(slice_chars(lines[FIRST_ROW_Y], column, 5), " +3.0");
    assert_eq!(slice_chars(lines[FIRST_ROW_Y + 1], column, 5), " -1.5");
}

/// 0 dB でも空欄にしない。空欄だと「auto gain が動いていない」と区別が付かない。
#[test]
fn a_zero_trim_is_still_printed() {
    let screen = screen_with_first_row(60, &[0]);

    let rendered = render_with_connection(&screen, &GridConnectionStatus::default());
    let lines = rendered.lines().collect::<Vec<_>>();

    assert_eq!(
        slice_chars(lines[FIRST_ROW_Y], gain_column(&screen), 5),
        " +0.0"
    );
}

/// 値を引くのは「いま鳴っている bank」の CLAP instance。待機 bank の値まで拾うと、
/// 差し替えの前に、まだ鳴っていないほうの数字が見えてしまう。
#[test]
fn the_standby_banks_gain_is_not_shown() {
    let screen = screen_with_first_row(60, &[0]);
    let standby = usize::from(screen.state.standby_instance_id(0));
    let connection = connection_with_gains(&[(standby, 5.0)]);

    let rendered = render_with_connection(&screen, &connection);
    let lines = rendered.lines().collect::<Vec<_>>();

    assert_eq!(
        slice_chars(lines[FIRST_ROW_Y], gain_column(&screen), 5),
        " +0.0"
    );
}
