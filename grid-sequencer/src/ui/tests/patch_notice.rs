//! patch selector を開けなかった理由の overlay。

use super::*;
use crate::patch_notice::{PatchNotice, PatchUnavailable};

/// 枠線が文字の間に入るので、空白を無視して探す。
fn contains_ignoring_spaces(rendered: &str, text: &str) -> bool {
    let needle = text.replace(' ', "");
    rendered
        .lines()
        .any(|line| line.replace(' ', "").contains(&needle))
}

fn render_notice(reason: PatchUnavailable) -> String {
    let mut screen = GridSequencerScreen::new(None);
    screen.patch_notice = Some(PatchNotice::new(reason, Instant::now()));
    render(&screen)
}

#[test]
fn the_unconfigured_notice_names_the_config_key_to_look_at() {
    let rendered = render_notice(PatchUnavailable::NotConfigured);

    assert!(
        contains_ignoring_spaces(&rendered, "音色を選べません"),
        "{rendered}"
    );
    assert!(
        contains_ignoring_spaces(&rendered, "patches_dirs が未設定です"),
        "{rendered}"
    );
}

#[test]
fn the_loading_notice_tells_the_user_to_retry_instead_of_reconfiguring() {
    let rendered = render_notice(PatchUnavailable::Loading);

    assert!(
        contains_ignoring_spaces(&rendered, "音色一覧を読み込み中です"),
        "{rendered}"
    );
}

#[test]
fn the_load_error_notice_shows_the_error_text() {
    let rendered = render_notice(PatchUnavailable::LoadError("catalog failed".to_string()));

    assert!(
        contains_ignoring_spaces(&rendered, "catalog failed"),
        "{rendered}"
    );
}

#[test]
fn the_poly_filter_notice_says_the_row_needs_poly_patches() {
    let rendered = render_notice(PatchUnavailable::NoPolyPatches);

    assert!(
        contains_ignoring_spaces(&rendered, "和音に使える音色がありません"),
        "{rendered}"
    );
    assert!(contains_ignoring_spaces(&rendered, "poly"), "{rendered}");
}

/// 通知は操作を塞がないので、消えるきっかけは時間だけ。
#[test]
fn the_notice_disappears_after_its_display_time() {
    let now = Instant::now();
    let mut screen = GridSequencerScreen::new(None);
    screen.patch_notice = Some(PatchNotice::new(PatchUnavailable::NoPatches, now));
    let ctx = crate::tests::ctx_with(
        crate::GridPatchLoad::Loading,
        crate::tests::empty_catalog(),
        &crate::NoVoicingLookup,
    );

    screen.pump_step(now + crate::patch_notice::PATCH_NOTICE_DURATION / 2, &ctx);
    assert!(screen.patch_notice_open());

    screen.pump_step(now + crate::patch_notice::PATCH_NOTICE_DURATION, &ctx);
    assert!(!screen.patch_notice_open());
}
