use super::*;
use crate::{LaneAddress, VisibleRowKind};

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
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 40), 5, 2, 5, false);
    assert_eq!(
        layout.hit_test(37, 4, &visible),
        Some(GridHit::NoteCell {
            address: LaneAddress::new(1, 1),
            step: 0,
        })
    );
    assert_eq!(
        layout.hit_test(68, 6, &visible),
        Some(GridHit::NoteCell {
            address: LaneAddress::new(1, 3),
            step: 15,
        })
    );
}

#[test]
fn both_characters_of_a_note_cell_hit_the_same_step() {
    let visible = rows(&[(0, 0)]);
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 20), 1, 1, 1, false);
    let expected = Some(GridHit::NoteCell {
        address: LaneAddress::new(0, 0),
        step: 0,
    });
    assert_eq!(layout.hit_test(37, 2, &visible), expected);
    assert_eq!(layout.hit_test(38, 2, &visible), expected);
}

#[test]
fn child_lane_patch_and_note_hits_keep_the_lane_ownership_rules() {
    let visible = rows(&[(0, 0), (1, 0), (1, 1)]);
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 20), 3, 2, 3, false);
    assert_eq!(
        layout.hit_test(7, 4, &visible),
        Some(GridHit::InstancePatch { instance: 1 })
    );
    assert_eq!(
        layout.hit_test(32, 4, &visible),
        Some(GridHit::LaneNote {
            address: LaneAddress::new(1, 1),
        })
    );
}

#[test]
fn chord_line_moves_the_grids_down_by_one_row() {
    let visible = rows(&[(0, 0)]);
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 20), 1, 1, 1, true);
    assert_eq!(layout.note.y, 1);
    assert_eq!(layout.hit_test(37, 2, &visible), None);
    assert!(layout.hit_test(37, 3, &visible).is_some());
}

#[test]
fn clipped_cells_and_rows_do_not_hit() {
    let visible = rows(&[(0, 0), (1, 0), (1, 1), (1, 2), (1, 3)]);
    let narrow = GridSequencerLayout::new(Rect::new(0, 0, 50, 20), 5, 2, 5, false);
    assert_eq!(narrow.hit_test(60, 2, &visible), None);

    let short = GridSequencerLayout::new(Rect::new(0, 0, 80, 6), 5, 2, 5, false);
    assert_eq!(short.hit_test(37, 5, &visible), None);
}

#[test]
fn note_cc1_and_velocity_heights_use_their_own_row_counts() {
    let layout = GridSequencerLayout::new(Rect::new(0, 0, 80, 30), 5, 2, 5, false);
    assert_eq!(layout.note.height, 8);
    assert_eq!(layout.cc1.height, 5);
    assert_eq!(layout.velocity.height, 8);
}
