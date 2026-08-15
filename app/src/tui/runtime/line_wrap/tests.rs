use super::*;

#[test]
fn disable_and_enable_emit_the_matching_terminal_commands() {
    let mut output = Vec::new();

    disable(&mut output).unwrap();
    enable(&mut output).unwrap();

    assert_eq!(output, b"\x1b[?7l\x1b[?7h");
}
