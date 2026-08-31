use super::*;

#[test]
fn adjusted_rows_are_named_with_one_based_numbers() {
    assert_eq!(describe_adjusted(&[1.0, 1.0, 1.0]), "none");
    assert_eq!(describe_adjusted(&[0.0, 1.0, 1.0]), "row1:mute");
    assert_eq!(describe_adjusted(&[1.0, 0.5, 2.0]), "row2:0.50x,row3:2.00x");
}
