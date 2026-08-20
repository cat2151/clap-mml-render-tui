use super::*;
use cmrt_tui_core::patch_plugins::PatchRoles;

#[test]
fn patch_name_wheel_uses_chord_candidates_only_on_the_chord_instance() {
    let patches = ["Bass/Mono.fxp", "Pads/Poly.fxp"]
        .into_iter()
        .map(|patch| (patch.to_string(), patch.to_lowercase()))
        .collect::<Vec<_>>();
    let plugins = crate::tests::plugins_with(PatchRoles {
        chord_patch_categories: vec!["Pads".to_string()],
        bass_patch_categories: vec!["Bass".to_string()],
        ..PatchRoles::default()
    });
    let mut ctx = context(&patches);
    ctx.patch_plugins = &plugins;
    // chord ON の行は 3=和音、4=bass、5〜8が 4 voice。
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.instances_mut()[0].patch = Some("Bass/Mono.fxp".to_string());
    screen.state.instances_mut()[1].patch = Some("Pads/Poly.fxp".to_string());
    screen.state.instances_mut()[2].patch = Some("Pads/Poly.fxp".to_string());
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    // chord summaryのPATCH wheelは、既存のカテゴリ+poly条件を満たす候補だけから選ぶ。
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, patch_column(&screen), 3),
        AREA,
        &ctx,
    );
    assert_eq!(
        screen.state.instances()[0].patch.as_deref(),
        Some("Pads/Poly.fxp")
    );

    // bass 行は bass 用カテゴリから引く。poly 判定は問わない。
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, patch_column(&screen), 4),
        AREA,
        &ctx,
    );
    assert_eq!(
        screen.state.instances()[1].patch.as_deref(),
        Some("Bass/Mono.fxp")
    );

    // 4 voiceのchild lane上でもinstance共有PATCHを変更する。送りは list なので下が次。
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, patch_column(&screen), 6),
        AREA,
        &ctx,
    );
    assert_eq!(
        screen.state.instances()[2].patch.as_deref(),
        Some("Bass/Mono.fxp")
    );
    assert!(!screen.cycle_random().patch);
}

/// アルペジオ行の PATCH wheel は arpeggio 用カテゴリからだけ引く。
/// chord mode 中は「chord 用カテゴリ以外」ではなくなるので、打楽器は候補に入らない。
#[test]
fn patch_name_wheel_uses_arpeggio_candidates_on_the_arpeggio_instance() {
    let patches = ["Percussion/Kick.fxp", "Leads/Mono.fxp", "Pads/Poly.fxp"]
        .into_iter()
        .map(|patch| (patch.to_string(), patch.to_lowercase()))
        .collect::<Vec<_>>();
    let plugins = crate::tests::plugins_with(PatchRoles {
        arpeggio_patch_categories: vec!["Leads".to_string()],
        ..PatchRoles::default()
    });
    let mut ctx = context(&patches);
    ctx.patch_plugins = &plugins;
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.instances_mut()[crate::ARPEGGIO_ROW].patch =
        Some("Percussion/Kick.fxp".to_string());
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    // 行3（4 voice）の PATCH 欄で wheel。行5〜8 のどの voice row からでも instance 単位。
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, patch_column(&screen), 6),
        AREA,
        &ctx,
    );

    assert_eq!(
        screen.state.instances()[crate::ARPEGGIO_ROW]
            .patch
            .as_deref(),
        Some("Leads/Mono.fxp")
    );
}

/// PATCH 欄の wheel は list 送り。下で次の音色へ進み、上で前に聴いた音色へ戻る。
/// 戻りきると wheel を回す前に鳴っていた音色そのものへ帰る。
#[test]
fn the_patch_name_wheel_walks_back_to_the_patch_it_started_from() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Keys/Alpha.fxp".to_string());
    // chord OFF なので NOTE grid の1行目（row 2）が instance 0。
    let scroll_down = mouse(MouseEventKind::ScrollDown, patch_column(&screen), 2);

    screen.handle_mouse(scroll_down, AREA, &ctx);
    let first = current_patch(&screen);
    screen.handle_mouse(scroll_down, AREA, &ctx);
    let second = current_patch(&screen);

    // Free 行は poly を避けるので候補は mono/未判定の2つ。1周のあいだ重複しない。
    assert_ne!(first, second);
    assert_ne!(first, "Keys/Alpha.fxp");

    screen.handle_mouse(
        mouse(MouseEventKind::ScrollUp, patch_column(&screen), 2),
        AREA,
        &ctx,
    );
    assert_eq!(current_patch(&screen), first);
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollUp, patch_column(&screen), 2),
        AREA,
        &ctx,
    );
    assert_eq!(current_patch(&screen), "Keys/Alpha.fxp");
}

/// chord mode を跨ぐと行の用途が変わり、候補も変わる。古い袋は捨てて引き直すので、
/// 戻れる先も「袋を作り直した時点で鳴っていた音色」に更新される。
#[test]
fn toggling_chord_mode_rebuilds_the_patch_bag_for_the_new_role() {
    let patches = patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.instances_mut()[crate::BASS_ROW].patch = Some("Keys/Alpha.fxp".to_string());

    // chord OFF の行2は Free。行3の PATCH 欄が instance 1。
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, patch_column(&screen), 3),
        AREA,
        &ctx,
    );
    let before_chord = bass_patch(&screen);
    assert_ne!(before_chord, "Keys/Alpha.fxp");

    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    // chord ON では bass 行が行4/5 の2 rowへ広がり、用途も Free から Bass へ変わる。
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, patch_column(&screen), 5),
        AREA,
        &ctx,
    );
    assert_ne!(bass_patch(&screen), before_chord);

    // 戻り先は Free の袋の履歴ではなく、作り直した袋の先頭。
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollUp, patch_column(&screen), 5),
        AREA,
        &ctx,
    );
    assert_eq!(bass_patch(&screen), before_chord);
    screen.handle_mouse(
        mouse(MouseEventKind::ScrollUp, patch_column(&screen), 5),
        AREA,
        &ctx,
    );
    assert_eq!(bass_patch(&screen), before_chord);
}

fn bass_patch(screen: &GridSequencerScreen) -> String {
    screen.state.instances()[crate::BASS_ROW]
        .patch
        .clone()
        .expect("bass 行に patch が入る")
}

fn current_patch(screen: &GridSequencerScreen) -> String {
    screen.state.rows()[0]
        .patch
        .clone()
        .expect("wheel が patch を当てている")
}

/// 用途の絞り込みで候補が 0 件になった wheel も、黙って何もしないのではなく理由を出す。
#[test]
fn a_wheel_with_no_candidates_for_the_role_reports_why_nothing_changed() {
    let patches = ["Bass/Mono.fxp"]
        .into_iter()
        .map(|patch| (patch.to_string(), patch.to_lowercase()))
        .collect::<Vec<_>>();
    let plugins = crate::tests::plugins_with(PatchRoles {
        chord_patch_categories: vec!["Pads".to_string()],
        ..PatchRoles::default()
    });
    let mut ctx = context(&patches);
    ctx.patch_plugins = &plugins;
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );

    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, patch_column(&screen), 3),
        AREA,
        &ctx,
    );

    assert_eq!(notice_reason(&screen), PatchUnavailable::NoRolePatches);
}

/// 一覧そのものが無いときは、用途ではなく一覧側の理由を出す（selector と同じ文面）。
/// `patches_dirs` が無ければ一覧も空になるので、両方を欠いた状態で確かめる。
#[test]
fn a_wheel_without_a_catalog_reports_the_catalog_reason() {
    let mut ctx = context(&[]);
    ctx.patch_dirs_configured = false;
    let mut screen = GridSequencerScreen::with_track_count(None, 2);

    screen.handle_mouse(
        mouse(MouseEventKind::ScrollDown, patch_column(&screen), 3),
        AREA,
        &ctx,
    );

    assert_eq!(notice_reason(&screen), PatchUnavailable::NotConfigured);
}
