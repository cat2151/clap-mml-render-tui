use super::*;

fn plain_lines(lines: &[Line<'static>]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn the_tree_help_lists_the_filter_key() {
    let (_, lines) = tree_help();
    let lines = plain_lines(&lines);

    let filter = lines
        .iter()
        .find(|line| line.contains("絞り込み"))
        .expect("tree のヘルプに絞り込みの行がない");
    assert!(filter.contains('/'), "{filter}");
    assert!(filter.contains("Esc"), "{filter}");
}

#[test]
fn the_tracks_help_does_not_list_the_filter_key() {
    let (_, lines) = tracks_help();

    assert!(plain_lines(&lines)
        .iter()
        .all(|line| !line.contains("絞り込み")));
}
