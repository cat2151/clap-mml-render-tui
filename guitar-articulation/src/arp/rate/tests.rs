use super::*;

#[test]
fn seconds_at_120_bpm() {
    let seconds: Vec<f64> = ArpRate::ALL.iter().map(|rate| rate.seconds(120)).collect();
    let expected = [0.25, 0.5 / 3.0, 0.125, 0.25 / 3.0, 0.0625];
    for (got, want) in seconds.iter().zip(expected) {
        assert!((got - want).abs() < 1e-12, "{seconds:?}");
    }
}

#[test]
fn stepped_walks_the_list_and_stops_at_its_ends() {
    assert_eq!(ArpRate::Sixteenth.stepped(1), ArpRate::SixteenthTriplet);
    assert_eq!(ArpRate::Sixteenth.stepped(-1), ArpRate::EighthTriplet);
    assert_eq!(ArpRate::ThirtySecond.stepped(1), ArpRate::ThirtySecond);
    assert_eq!(ArpRate::Eighth.stepped(-1), ArpRate::Eighth);
}

#[test]
fn labels_round_trip_and_an_unknown_label_reads_as_sixteenth() {
    for rate in ArpRate::ALL {
        let json = serde_json::to_string(&rate).unwrap();
        assert_eq!(json, format!("\"{}\"", rate.label()));
        assert_eq!(serde_json::from_str::<ArpRate>(&json).unwrap(), rate);
    }
    assert_eq!(
        serde_json::from_str::<ArpRate>("\"4\"").unwrap(),
        ArpRate::Sixteenth
    );
}
