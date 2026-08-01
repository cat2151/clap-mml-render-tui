use super::*;

#[test]
fn boosted_rows_are_named_with_one_based_numbers() {
    assert_eq!(describe_boosted(&[0.0, 0.0, 0.0]), "none");
    assert_eq!(describe_boosted(&[6.0, 0.0, 0.0]), "row1:+6dB");
    assert_eq!(describe_boosted(&[0.0, -6.0, 6.0]), "row2:-6dB,row3:+6dB");
}
