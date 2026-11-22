use super::*;
use cmrt_core::PluginKey;

#[test]
fn assessment_round_trip_preserves_boolean_and_evidence() {
    use AudioClassification as Audio;
    use RoutingClassification as Routing;
    let routing = |classification| Observation::Routing {
        plugin_version: "1.0".into(),
        classification,
        evidence: "CC1 routing inspected".into(),
    };
    let audio = |classification| Observation::Audio {
        plugin_version: "1.0".into(),
        routing_unresolved: "Unknown patch encoding".into(),
        conditions: Conditions {
            sample_rate_hz: 48_000,
            duration_seconds: 2,
            block_samples: 512,
            preroll_blocks: 10,
            note_number: 60,
            velocity: 100,
            cc1_values: [0, 127],
            fresh_instance_per_cc: true,
            analysis: "nsdf_pitch_comparison_v2".into(),
        },
        classification,
        frame_samples: 4096,
        trials: [None, None],
    };
    let cases = [
        (routing(Routing::Vibrato), true),
        (routing(Routing::FixedPitchChange), true),
        (routing(Routing::NonPitchRouting), false),
        (routing(Routing::Unconnected), false),
        (audio(Audio::VibratoIncreaseCandidate), true),
        (audio(Audio::VibratoDecreaseCandidate), true),
        (audio(Audio::FixedPitchChangeCandidate), true),
        (audio(Audio::NoClearPitchChange), false),
        (audio(Audio::PitchUncertain), false),
        (
            Observation::MeasurementFailed {
                routing_unresolved: "Unknown patch encoding".into(),
                plugin_version: Some("1.0".into()),
                error: "CC1=0: Audio render failed".into(),
            },
            false,
        ),
    ];
    for (observation, expected) in cases {
        let patch = PatchRef {
            plugin: PluginKey::from_identity(Some("test.plugin"), ""),
            display: "patch.fxp".into(),
        };
        let record = Record::new(patch.clone(), observation);
        let json = serde_json::to_value(record).unwrap();
        assert_eq!(json["pitch_change_likely"].as_bool(), Some(expected));
        assert!(json["observation"]["method"].is_string());
        let restored: Record = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(restored.patch, patch);
        assert_eq!(restored.observation.pitch_change_likely(), expected);
        assert_eq!(serde_json::to_value(restored).unwrap(), json);
    }
}
