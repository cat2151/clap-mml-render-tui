use std::collections::BTreeSet;

use super::*;

/// METAL-GTX Full の KS のうち、この画面から出すもの。オルタネイト（13 / 16 / 19 / 22）と
/// PBR12 / PBR24（97 / 98）は含めない。
const EXPECTED_KEYSWITCHES: [u8; 32] = [
    4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 17, 18, 20, 21, 23, 24, 25, 26, 27, 28, 29, 91, 92, 93,
    94, 95, 96, 99, 100, 101, 102,
];

#[test]
fn the_keyswitch_set_is_fixed() {
    let actual: BTreeSet<u8> = Articulation::ALL.iter().map(|a| a.keyswitch()).collect();
    let expected: BTreeSet<u8> = EXPECTED_KEYSWITCHES.into_iter().collect();
    assert_eq!(actual, expected);
    assert_eq!(
        actual.len(),
        Articulation::ALL.len(),
        "KS 番号が重複している"
    );
}
