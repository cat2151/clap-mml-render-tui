use super::*;

/// 狭い端末で 2 行目以降が切れても用が足りるよう、1 行目だけで何が起きたか分かること。
#[test]
fn every_reason_leads_with_a_line_that_stands_on_its_own() {
    for reason in [
        PatchUnavailable::NotConfigured,
        PatchUnavailable::Loading,
        PatchUnavailable::LoadError("catalog failed".to_string()),
        PatchUnavailable::NoPatches,
        PatchUnavailable::NoPolyPatches,
        PatchUnavailable::NoRolePatches,
    ] {
        let lines = reason.lines();
        let first = lines.first().expect("理由には必ず 1 行以上ある");
        assert!(!first.trim().is_empty(), "{reason:?}");
        assert!(
            first.contains("音色") || first.contains("和音"),
            "{reason:?}"
        );
    }
}

#[test]
fn the_notice_expires_only_after_its_display_time() {
    let now = Instant::now();
    let notice = PatchNotice::new(PatchUnavailable::NoPatches, now);

    assert!(!notice.expired(now));
    assert!(!notice.expired(now + PATCH_NOTICE_DURATION - Duration::from_millis(1)));
    assert!(notice.expired(now + PATCH_NOTICE_DURATION));
}
