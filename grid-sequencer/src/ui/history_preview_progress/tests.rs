use super::*;

#[test]
fn progress_uses_completed_tracks_and_whole_elapsed_seconds() {
    let lines = progress_lines(5, 7, Duration::from_millis(12_999));
    let rendered = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("render 5/7 tracks"), "{rendered}");
    assert!(rendered.contains("elapsed 12s"), "{rendered}");
}
