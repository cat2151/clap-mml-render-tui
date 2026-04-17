use super::*;

#[test]
fn parse_get_mml_query_accepts_measure_alias_and_zero() {
    assert_eq!(parse_get_mml_query("/mml?track=2&measure=0"), Ok((2, 0)));
    assert_eq!(parse_get_mml_query("/mml?track=2&meas=0"), Ok((2, 0)));
}

#[test]
fn parse_get_mml_query_rejects_missing_or_invalid_values() {
    assert_eq!(
        parse_get_mml_query("/mml?track=2"),
        Err((400, "track と measure を指定してください\n".to_string()))
    );
    assert_eq!(
        parse_get_mml_query("/mml?track=&measure=0"),
        Err((400, "track を指定してください\n".to_string()))
    );
    assert_eq!(
        parse_get_mml_query("/mml?track=2&measure="),
        Err((400, "measure を指定してください\n".to_string()))
    );
    assert_eq!(
        parse_get_mml_query("/mml?track=abc&measure=0"),
        Err((400, "track は 0 以上の整数を指定してください\n".to_string()))
    );
    assert_eq!(
        parse_get_mml_query("/mml?track=2&measure=abc"),
        Err((
            400,
            "measure は 0 以上の整数を指定してください\n".to_string()
        ))
    );
    assert_eq!(
        parse_get_mml_query("/mml?track=2&meas=abc"),
        Err((
            400,
            "measure は 0 以上の整数を指定してください\n".to_string()
        ))
    );
}

#[test]
fn get_snapshot_mml_rejects_unready_and_out_of_range_requests() {
    let state = DawHttpState::default();
    assert_eq!(
        get_snapshot_mml(&state, 0, 0),
        Err((503, "DAW データの準備中です\n".to_string()))
    );

    let state = DawHttpState {
        cfg: None,
        pending_commands: VecDeque::new(),
        grid_snapshot: vec![vec!["t120".to_string()]],
    };
    assert_eq!(
        get_snapshot_mml(&state, 1, 0),
        Err((
            404,
            "指定された track/measure は範囲外です: track=1, measure=0\n".to_string()
        ))
    );
    assert_eq!(get_snapshot_mml(&state, 0, 0), Ok("t120".to_string()));
}

#[test]
fn get_snapshot_mmls_rejects_unready_state_and_returns_all_tracks_measures() {
    let state = DawHttpState::default();
    assert_eq!(
        get_snapshot_mmls(&state),
        Err((503, "DAW データの準備中です\n".to_string()))
    );

    let state = DawHttpState {
        cfg: None,
        pending_commands: VecDeque::new(),
        grid_snapshot: vec![
            vec!["t120".to_string(), String::new()],
            vec!["@1".to_string(), "l8cde".to_string()],
        ],
    };
    assert_eq!(
        get_snapshot_mmls(&state),
        Ok(vec![
            vec!["t120".to_string(), String::new()],
            vec!["@1".to_string(), "l8cde".to_string()],
        ])
    );
}

#[test]
fn snapshot_mmls_etag_is_content_based() {
    let tracks = vec![
        vec!["t120".to_string(), String::new()],
        vec!["@1".to_string(), "l8cde".to_string()],
    ];
    let same_tracks = tracks.clone();
    let different_tracks = vec![
        vec!["t120".to_string(), String::new()],
        vec!["@1".to_string(), "l8cdef".to_string()],
    ];

    assert_eq!(
        snapshot_mmls_etag(&tracks),
        snapshot_mmls_etag(&same_tracks)
    );
    assert_ne!(
        snapshot_mmls_etag(&tracks),
        snapshot_mmls_etag(&different_tracks)
    );
}

#[test]
fn if_none_match_matches_exact_weak_and_wildcard_etags() {
    let etag = snapshot_mmls_etag(&[vec!["l8cde".to_string()]]);

    assert!(if_none_match_matches(&etag, &etag));
    assert!(if_none_match_matches(&format!("W/{etag}"), &etag));
    assert!(if_none_match_matches("*", &etag));
    assert!(!if_none_match_matches("\"different\"", &etag));
}
