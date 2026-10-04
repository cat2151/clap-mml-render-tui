use std::time::{Duration, Instant};

use super::note_columns::{cells, chord_state, cycle_to, lit, names, render_at, TICK};
use crate::NotePlaybackMode;

#[test]
fn a_shorter_progression_switches_its_rows_and_highlights_only_at_the_deadline() {
    for mode in [NotePlaybackMode::Repeat, NotePlaybackMode::Arp] {
        for after in [false, true] {
            let origin = Instant::now();
            let mut state = chord_state(vec![vec![60, 64], vec![65, 69]], origin);
            cycle_to(&mut state, mode, origin);
            let boundary_step = if mode == NotePlaybackMode::Repeat {
                16
            } else {
                8
            };
            let deadline = origin + TICK * boundary_step;
            for step in 1..=boundary_step {
                state
                    .poll_periodic_tick_with_progression(origin + TICK * step, |_| {
                        Some(vec![vec![61]])
                    })
                    .unwrap();
            }
            let now = if after {
                deadline
            } else {
                deadline - Duration::from_millis(1)
            };
            let terminal = render_at(state, now);
            let label = if mode == NotePlaybackMode::Repeat {
                "│Repeat: "
            } else {
                "│Arp: "
            };
            let status = cells(&terminal, label);
            let columns = cells(&terminal, "│Note:    ");
            if after {
                assert_eq!(lit(&status), ["C#4"]);
                assert_eq!(lit(&columns), ["C#4"]);
                if mode == NotePlaybackMode::Repeat {
                    assert_eq!(names(&status), ["C#4"]);
                } else {
                    assert_eq!(names(&status), ["C#4", "C#5"]);
                }
            } else if mode == NotePlaybackMode::Repeat {
                assert_eq!(names(&status), ["C4", "E4", "|", "F4", "A4"]);
                assert_eq!(lit(&status), ["F4", "A4"]);
                assert_eq!(lit(&columns), ["F4", "A4"]);
            } else {
                assert_eq!(
                    names(&status),
                    ["C4", "E4", "C5", "E5", "|", "F4", "A4", "F5", "A5"]
                );
                assert_eq!(lit(&status), ["A5"]);
                assert_eq!(lit(&columns), ["A5"]);
            }
        }
    }
}
