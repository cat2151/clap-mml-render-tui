use super::*;
use crate::{LaneAddress, VisibleRowKind};

/// 右 pane を出さない構成（section が1つも無い）。
const NO_LIST: &[usize] = &[];
/// chord mode の構成。arp（見出し + 9型）と bass（見出し + 6型）。
const CHORD_LIST: &[usize] = &[1 + 9, 1 + 6];

fn rows(addresses: &[(usize, usize)]) -> Vec<VisibleNoteRow> {
    addresses
        .iter()
        .map(|&(instance, lane)| VisibleNoteRow {
            address: LaneAddress::new(instance, lane),
            kind: VisibleRowKind::Normal,
        })
        .collect()
}

#[test]
fn note_cells_resolve_flat_rows_through_the_shared_mapping() {
    let visible = rows(&[(0, 0), (1, 0), (1, 1), (1, 2), (1, 3)]);
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 40), 5, 2, 5, false, NO_LIST);
    assert_eq!(
        layout.hit_test(layout.step_column(0), 4, &visible),
        Some(GridHit::NoteCell {
            address: LaneAddress::new(1, 1),
            step: 0,
        })
    );
    assert_eq!(
        layout.hit_test(layout.step_column(15), 6, &visible),
        Some(GridHit::NoteCell {
            address: LaneAddress::new(1, 3),
            step: 15,
        })
    );
}

#[test]
fn both_characters_of_a_note_cell_hit_the_same_step() {
    let visible = rows(&[(0, 0)]);
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 20), 1, 1, 1, false, NO_LIST);
    let expected = Some(GridHit::NoteCell {
        address: LaneAddress::new(0, 0),
        step: 0,
    });
    assert_eq!(
        layout.hit_test(layout.step_column(0), 2, &visible),
        expected
    );
    assert_eq!(
        layout.hit_test(layout.step_column(0) + 1, 2, &visible),
        expected
    );
}

#[test]
fn child_lane_patch_and_note_hits_keep_the_lane_ownership_rules() {
    let visible = rows(&[(0, 0), (1, 0), (1, 1)]);
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 20), 3, 2, 3, false, NO_LIST);
    assert_eq!(
        layout.hit_test(layout.patch_column() + 1, 4, &visible),
        Some(GridHit::InstancePatch { instance: 1 })
    );
    assert_eq!(
        layout.hit_test(layout.note_column() + 1, 4, &visible),
        Some(GridHit::LaneNote {
            address: LaneAddress::new(1, 1),
        })
    );
}

#[test]
fn chord_line_moves_the_grids_down_by_one_row() {
    let visible = rows(&[(0, 0)]);
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 20), 1, 1, 1, true, NO_LIST);
    assert_eq!(layout.note.y, 1);
    assert_eq!(layout.hit_test(layout.step_column(0), 2, &visible), None);
    assert!(layout
        .hit_test(layout.step_column(0), 3, &visible)
        .is_some());
}

#[test]
fn clipped_cells_and_rows_do_not_hit() {
    let visible = rows(&[(0, 0), (1, 0), (1, 1), (1, 2), (1, 3)]);
    let narrow = GridSequencerLayout::new(Rect::new(0, 0, 50, 20), 5, 2, 5, false, NO_LIST);
    assert_eq!(narrow.hit_test(60, 2, &visible), None);

    let short = GridSequencerLayout::new(Rect::new(0, 0, 80, 6), 5, 2, 5, false, NO_LIST);
    assert_eq!(short.hit_test(short.step_column(0), 5, &visible), None);
}

#[test]
fn note_cc1_and_velocity_heights_use_their_own_row_counts() {
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 30), 5, 2, 5, false, NO_LIST);
    assert_eq!(layout.note.height, 8);
    assert_eq!(layout.cc1.height, 5);
    assert_eq!(layout.velocity.height, 8);
}

#[test]
fn patch_load_progress_sits_directly_below_the_note_tracks() {
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 30), 5, 2, 5, false, NO_LIST);

    assert_eq!(layout.patch_load_progress.height, 1);
    assert_eq!(
        layout.patch_load_progress.y,
        layout.note.y + layout.note.height
    );
    assert_eq!(layout.patch_load_progress.x, layout.note.x);
    assert_eq!(layout.patch_load_progress.width, layout.note.width);
}

/// 端末がいくら広くても grid は中身ぶんの幅しか取らず、塊ごと中央へ寄る。
#[test]
fn the_grid_keeps_its_content_width_and_the_block_is_centred() {
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 200, 30), 5, 2, 5, true, CHORD_LIST);
    let pane = layout.pattern_list.expect("200桁なら list が入る");
    let block_width = GRID_WIDTH + PATTERN_LIST_WIDTH;

    assert_eq!(layout.note.width, GRID_WIDTH);
    assert_eq!(layout.note.x, (200 - block_width) / 2);
    // grid と list の間に隙間を作らない。
    assert_eq!(pane.x, layout.note.x + GRID_WIDTH);
    assert_eq!(pane.x + pane.width, layout.note.x + block_width);
    // 左右の余白はほぼ同じ幅になる。余りが奇数のときだけ右が1桁広い。
    let right = 200 - (pane.x + pane.width);
    assert!(
        layout.note.x.abs_diff(right) <= 1,
        "{} vs {right}",
        layout.note.x
    );
    // コード行も塊に揃える。
    let chord_line = layout.chord_line.expect("chord mode 中は出る");
    assert_eq!(chord_line.x, layout.note.x);
    assert_eq!(chord_line.width, block_width);
}

/// 縦に広い端末でも右 pane は中身ぶんの高さで止める。grid と同じ高さへ伸ばすと、
/// list の下に長い空白が伸びるだけになる。
#[test]
fn the_phrase_list_pane_is_trimmed_to_its_content_height() {
    // コード行1 + ステータス1 + キーバインド1 を除いた残りが grid と list の領域。
    let rows_height = 60 - 3;
    let tall = GridSequencerLayout::new(Rect::new(0, 0, 200, 60), 5, 2, 5, true, CHORD_LIST);
    let pane = tall.pattern_list.expect("200桁なら list が入る");

    assert_eq!(pane.height, content_height(u16::MAX));
    assert!(pane.height < rows_height, "{pane:?}");
    // 上端は grid と揃えたまま。詰めるのは下だけ。
    assert_eq!(pane.y, tall.note.y);
}

/// pane に割ける高さが中身より低ければ、そこからはみ出させない。
#[test]
fn the_phrase_list_pane_never_exceeds_the_rows_area() {
    let rows_height = 12 - 3;
    let short = GridSequencerLayout::new(Rect::new(0, 0, 200, 12), 5, 2, 5, true, CHORD_LIST);
    let pane = short.pattern_list.expect("200桁なら list が入る");

    assert!(pane.height <= rows_height, "{pane:?}");
    assert_eq!(pane.height, content_height(rows_height));
}

fn content_height(available: u16) -> u16 {
    crate::ui::pattern_list::height_for(CHORD_LIST, available)
}

/// ステータスとキーバインドは中央寄せに巻き込まない。詰めると末尾から情報が欠ける。
#[test]
fn the_bottom_lines_keep_the_whole_terminal_width() {
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 200, 30), 5, 2, 5, true, CHORD_LIST);

    for line in [layout.status, layout.keybind] {
        assert_eq!(line.x, 0);
        assert_eq!(line.width, 200);
    }
}

/// 出す section が無ければ list を出さず、grid だけを中央へ置く。
#[test]
fn the_phrase_list_pane_only_appears_when_a_section_is_shown() {
    let width = GRID_WIDTH + PATTERN_LIST_WIDTH;
    let area = Rect::new(0, 0, width, 30);
    let off = GridSequencerLayout::new(area, 5, 2, 5, false, NO_LIST);
    assert_eq!(off.pattern_list, None);
    assert_eq!(off.note.width, GRID_WIDTH);
    assert_eq!(off.note.x, (width - GRID_WIDTH) / 2);

    let on = GridSequencerLayout::new(area, 5, 2, 5, true, CHORD_LIST);
    let pane = on
        .pattern_list
        .expect("塊がちょうど収まる幅なら list が入る");
    assert_eq!(pane.width, PATTERN_LIST_WIDTH);
    // 値 grid も NOTE grid と同じ幅・同じ左端で縦に揃える。
    for grid in [on.note, on.cc1, on.velocity] {
        assert_eq!(grid.width, GRID_WIDTH);
        assert_eq!(grid.x, on.note.x);
    }
}

#[test]
fn the_phrase_list_uses_one_column_at_the_previous_minimum_width() {
    let width = GRID_WIDTH + PATTERN_LIST_MIN_WIDTH;
    let layout = GridSequencerLayout::new(Rect::new(0, 0, width, 30), 5, 2, 5, true, CHORD_LIST);

    assert_eq!(
        layout
            .pattern_list
            .expect("minimum width still shows the pane")
            .width,
        PATTERN_LIST_MIN_WIDTH
    );
}

/// 16 step ぶんを削ってまでは出さない。grid の幅が最優先。
#[test]
fn a_terminal_too_narrow_for_both_keeps_the_grid_and_drops_the_pane() {
    let visible = rows(&[(0, 0)]);
    let width = GRID_WIDTH + PATTERN_LIST_MIN_WIDTH - 1;
    let layout = GridSequencerLayout::new(Rect::new(0, 0, width, 20), 1, 1, 1, true, CHORD_LIST);

    assert_eq!(layout.pattern_list, None);
    assert_eq!(layout.note.width, GRID_WIDTH);
    // 削られていないので最終 step も当たる。
    assert_eq!(
        layout.hit_test(layout.step_column(15), 3, &visible),
        Some(GridHit::NoteCell {
            address: LaneAddress::new(0, 0),
            step: 15,
        })
    );
}

/// grid より狭い端末では中央寄せをあきらめ、左端から幅を全部使う。
#[test]
fn a_terminal_narrower_than_the_grid_uses_every_column() {
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 50, 20), 1, 1, 1, true, CHORD_LIST);

    assert_eq!(layout.note.x, 0);
    assert_eq!(layout.note.width, 50);
}

/// list の有無で step セルの当たり判定が動かないこと。塊の幅が変わるぶん
/// grid の左端は動くので、比べるのは列そのものではなく grid からの相対位置。
#[test]
fn the_pane_does_not_move_any_note_cell_relative_to_the_grid() {
    let visible = rows(&[(0, 0)]);
    let area = Rect::new(0, 0, GRID_WIDTH + PATTERN_LIST_WIDTH, 20);
    let without = GridSequencerLayout::new(area, 1, 1, 1, true, NO_LIST);
    let with = GridSequencerLayout::new(area, 1, 1, 1, true, CHORD_LIST);

    for step in [0, 1, 15] {
        assert_eq!(
            with.hit_test(with.step_column(step), 3, &visible),
            without.hit_test(without.step_column(step), 3, &visible),
            "step {step}"
        );
    }
}
