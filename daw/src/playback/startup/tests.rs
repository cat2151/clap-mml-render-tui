use super::*;

#[test]
fn nothing_is_shown_before_a_play_starts() {
    assert!(DawPlaybackStartupState::default().snapshot().is_none());
}

#[test]
fn the_first_measure_stage_keeps_the_start_time_of_the_play() {
    let state = DawPlaybackStartupState::default();
    state.begin(true);
    let started = state.snapshot().unwrap().started;

    state.begin_first_measure(7);

    let snapshot = state.snapshot().unwrap();
    assert_eq!(snapshot.started, started);
    assert_eq!(
        snapshot.stage,
        DawPlaybackStartupStage::FirstMeasure {
            loaded: 0,
            total: 7
        }
    );
}

#[test]
fn loaded_tracks_are_counted_up() {
    let state = DawPlaybackStartupState::default();
    state.begin(true);
    state.begin_first_measure(7);

    state.note_measure_loaded(3);

    assert_eq!(
        state.snapshot().unwrap().stage,
        DawPlaybackStartupStage::FirstMeasure {
            loaded: 3,
            total: 7
        }
    );
}

/// 演奏を止めたら overlay は消える。止めたあとに遅れて届いた報告で
/// 出し直してはいけない（音は鳴らないのに「読み込み中」が残る）。
#[test]
fn a_late_report_after_finish_does_not_bring_the_overlay_back() {
    let state = DawPlaybackStartupState::default();
    state.begin(true);
    state.begin_first_measure(7);
    state.finish();

    state.note_measure_loaded(5);

    assert!(state.snapshot().is_none());
}

/// play server の起動待ちの最中に届いた報告は無視する。段階が違うので、
/// ここで書けてしまうと 1 小節目のロードが始まる前に進捗が進んで見える。
#[test]
fn a_report_while_waiting_for_the_server_is_ignored() {
    let state = DawPlaybackStartupState::default();
    state.begin(true);

    state.note_measure_loaded(4);

    assert_eq!(
        state.snapshot().unwrap().stage,
        DawPlaybackStartupStage::PlayServer {
            first_measure_follows: true
        }
    );
}
