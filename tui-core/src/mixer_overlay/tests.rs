use super::*;

fn track(label: &str, role: Option<&str>, patch: Option<&str>) -> MixerOverlayTrack {
    MixerOverlayTrack {
        label: label.to_string(),
        volume_db: 0,
        role: role.map(str::to_string),
        patch: patch.map(str::to_string),
    }
}

fn tracks_with_patch(count: usize) -> Vec<MixerOverlayTrack> {
    (0..count)
        .map(|i| {
            track(
                &format!("T{}", i + 1),
                Some("lead"),
                Some("Reed To Pipe Morph"),
            )
        })
        .collect()
}

#[test]
fn wide_terminals_widen_the_column_so_patch_names_stay_readable() {
    // 内幅 108（幅 120 の端末）で 8 track なら 1 列 12 桁まで取れる。
    assert_eq!(track_column_width(108, 8), 12);
}

#[test]
fn narrow_terminals_fall_back_to_the_minimum_column_width() {
    // 内幅 71（幅 80 の端末）では 8 桁。ここから先は横スクロールで送る。
    assert_eq!(track_column_width(71, 8), MIN_TRACK_COLUMN_WIDTH);
    assert_eq!(track_column_width(20, 8), MIN_TRACK_COLUMN_WIDTH);
}

#[test]
fn the_column_never_grows_past_the_maximum_width() {
    assert_eq!(track_column_width(400, 2), MAX_TRACK_COLUMN_WIDTH);
}

#[test]
fn a_zero_track_count_does_not_divide_by_zero() {
    assert_eq!(track_column_width(108, 0), MAX_TRACK_COLUMN_WIDTH);
}

#[test]
fn narrow_columns_scroll_instead_of_shrinking_below_the_minimum() {
    let inner = Rect::new(0, 0, 71, 20);
    let range = visible_track_range(8, 7, inner, MIN_TRACK_COLUMN_WIDTH);
    // 7 + 8*8 = 71 なので 8 track ちょうど収まる。
    assert_eq!(range, 0..8);

    let narrow = Rect::new(0, 0, 40, 20);
    let range = visible_track_range(8, 7, narrow, MIN_TRACK_COLUMN_WIDTH);
    assert!(range.len() < 8, "range: {range:?}");
    assert!(
        range.contains(&7),
        "選択中の track は必ず見えること: {range:?}"
    );
}

#[test]
fn tracks_without_patch_info_keep_the_single_line_header() {
    let tracks = vec![track("track1", None, None), track("track2", None, None)];
    assert_eq!(header_line_count(&tracks, 20), HEADER_LINES_PLAIN);
}

#[test]
fn tracks_with_patch_info_get_the_role_and_patch_header_lines() {
    assert_eq!(
        header_line_count(&tracks_with_patch(4), 20),
        HEADER_LINES_WITH_PATCH
    );
}

#[test]
fn a_short_overlay_drops_back_to_the_single_line_header() {
    // 高さが無いときにヘッダを 3 行取ると、メーターが消えて何も読めなくなる。
    assert_eq!(
        header_line_count(&tracks_with_patch(4), MIN_HEIGHT_FOR_PATCH_HEADER - 1),
        HEADER_LINES_PLAIN
    );
}

#[test]
fn a_column_cell_is_left_aligned_and_keeps_one_space_of_separation() {
    assert_eq!(column_cell("T1", 8), "T1      ");
    // 列幅ちょうどで切らず、区切りの 1 桁を必ず残す。
    assert_eq!(column_cell("909 Kick Long", 8), "909 Kic ");
    assert_eq!(column_cell("909 Kick Long", 12), "909 Kick Lo ");
}

#[test]
fn a_column_cell_counts_characters_not_bytes() {
    // 切り詰めは byte 境界ではなく文字境界で行う（multi-byte で panic させない）。
    assert_eq!(column_cell("ピアノ音色フルート", 8), "ピアノ音色フル ");
}
