//! 汚しが OFF のとき、変換の出力が固定値のままであること。

use super::*;
use crate::{convert, Rule};

/// 汚しが OFF のときの [`convert`] の出力。1 行 1 イベントで `秒 status note value`（16 進）。
/// 頭の 4 行は列で送る CC（CC20 / CC23 / CC32 / CC24）の既定値。
const ECO_MELODY: &str = "\
    0.0 b01400\n\
    0.0 b01700\n\
    0.0 b02000\n\
    0.0 b0180d\n\
    0.0 90117f\n\
    0.0 90287f\n\
    0.25 801100\n\
    0.25 802800\n\
    0.25 90127f\n\
    0.25 902a5f\n\
    0.5 801200\n\
    0.5 802a00\n\
    0.5 90117f\n\
    0.5 902b5f\n\
    0.75 801100\n\
    0.75 802b00\n\
    0.75 902d7f\n\
    1.0 802d00\n\
    1.0 90127f\n\
    1.0 902b5f\n\
    1.25 801200\n\
    1.25 802b00\n\
    1.25 90117f\n\
    1.25 902a5f\n\
    1.5 801100\n\
    1.5 802a00\n\
";

const CHORD_WITH_ECO: &str = "\
    0.0 b01400\n\
    0.0 b01700\n\
    0.0 b02000\n\
    0.0 b0180d\n\
    0.0 90117f\n\
    0.0 90307f\n\
    0.125 801100\n\
    0.125 803000\n\
    0.125 90305f\n\
    0.125 90345f\n\
    0.125 90375f\n\
    0.25 803000\n\
    0.25 803400\n\
    0.25 803700\n\
    0.25 90325f\n\
    0.375 803200\n\
    0.375 90127f\n\
    0.375 90347f\n\
    0.5 801200\n\
    0.5 803400\n\
    0.5 90117f\n\
    0.5 90325f\n\
    0.625 801100\n\
    0.625 803200\n\
    0.625 90127f\n\
    0.625 90305f\n\
    0.75 801200\n\
    0.75 803000\n\
    0.75 90117f\n\
    0.8 801100\n\
";

const COLUMN_RULES: &str = "\
    0.0 b01400\n\
    0.0 b01700\n\
    0.0 b02000\n\
    0.0 b0180d\n\
    0.0 90117f\n\
    0.0 90287f\n\
    0.25 801100\n\
    0.25 802800\n\
    0.25 901a7f\n\
    0.25 902b7f\n\
    0.5 801a00\n\
    0.5 802b00\n\
    0.5 b01a18\n\
    0.5 90187f\n\
    0.5 902d7f\n\
    0.75 801800\n\
    0.75 802d00\n\
    0.75 b01440\n\
    0.75 90117f\n\
    0.75 90287f\n\
    1.0 801100\n\
    1.0 802800\n\
    1.0 b01a13\n\
    1.0 b01400\n\
    1.0 b01400\n\
";

const REPEATED_PITCH: &str = "\
    0.0 b01400\n\
    0.0 b01700\n\
    0.0 b02000\n\
    0.0 b0180d\n\
    0.0 90117f\n\
    0.0 90307f\n\
    0.125 801100\n\
    0.125 803000\n\
    0.125 90307f\n\
    0.25 803000\n\
    0.25 90307f\n\
    0.375 803000\n\
    0.375 90307f\n\
    0.5 803000\n\
";

fn dump(mml: &str, rules: &RuleTable) -> String {
    let events = cmrt_chord::timed_performance(mml).unwrap().events;
    convert(&events, rules)
        .iter()
        .map(|e| {
            let [status, key, value] = e.message;
            format!("{:?} {status:02x}{key:02x}{value:02x}\n", e.seconds)
        })
        .collect()
}

#[test]
fn without_the_rule_the_output_is_unchanged() {
    let mut columns = RuleTable::default();
    columns.toggle(1, Rule::HammerPull);
    columns.toggle(2, Rule::Slide);
    columns.toggle(3, Rule::Vibrato);
    let cases = [
        (
            "o3 l8 e f+ g a g f+",
            rules(&[RowRule::EconomyPicking]),
            ECO_MELODY,
        ),
        (
            "t120 o4 l16 c 'ceg' d e d c",
            rules(&[RowRule::EconomyPicking]),
            CHORD_WITH_ECO,
        ),
        ("o3 l8 e g a e", columns, COLUMN_RULES),
        ("o4 l16 c c c c", RuleTable::default(), REPEATED_PITCH),
    ];
    for (mml, rules, expected) in cases {
        assert_eq!(dump(mml, &rules), expected, "{mml}");
    }
}
