use super::*;

#[test]
fn surge_preset_routes_include_muting_and_unresolved_targets() {
    use RoutingClassification::*;
    let inspect_fixture = |params: &str, extra: &str| {
        let xml =
            format!("<patch revision=\"20\"><parameters>{params}</parameters>{extra}</patch>");
        inspect(&Element::parse(xml.as_bytes()).unwrap())
    };
    let route = |target: &str, extra: &str| {
        format!("<{target}><modrouting source=\"6\" depth=\"1.5\" {extra}/></{target}>")
    };
    let pitch = route("a_osc1_pitch", "");
    let filter = route("b_filter2_cutoff", "");
    assert!(matches!(
        inspect_fixture(&pitch, "").unwrap().0,
        FixedPitchChange
    ));
    assert!(matches!(
        inspect_fixture(&filter, "").unwrap().0,
        NonPitchRouting
    ));
    assert!(matches!(
        inspect_fixture(&(pitch + &filter), "").unwrap().0,
        FixedPitchChange
    ));
    assert!(matches!(
        inspect_fixture(&route("a_pitch", "muted=\"1\""), "")
            .unwrap()
            .0,
        Unconnected
    ));
    assert!(inspect_fixture(&route("a_lfo1_magnitude", ""), "").is_err());
    assert!(inspect_fixture("", "<formulae><formula/></formulae>").is_err());
    assert!(inspect_fixture("", "<dawExtraState><midictrl_map/></dawExtraState>").is_err());
}
