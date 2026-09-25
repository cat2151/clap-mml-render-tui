//! 「準備済みか」は音色と effect chain の両方で決まる。
//!
//! server は chain 無しの準備を同じ音色で受けると chain を外す。音色だけで比べると、
//! chain 付きの instance を chain 無しの行へ使い回し（chain が残る）、その逆では
//! chain を載せ直さない（chain が外れたまま）。

use super::*;

fn with_chain(name: &str, chain: &str) -> LivePatch {
    LivePatch::with_effect_chain(Some(name), chain)
}

const REVERB: &str = r#"[{"Surge XT Effects preset":"Reverb 1/Cathedral 2.srgfx"}]"#;

#[test]
fn the_same_patch_with_another_chain_is_not_ready() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice
        .prepare(&sink, MML_OVERLAY_INSTANCE, &with_chain("a.fxp", REVERB))
        .unwrap();

    assert!(voice.is_patch_ready(MML_OVERLAY_INSTANCE, &with_chain("a.fxp", REVERB)));
    assert!(!voice.is_patch_ready(MML_OVERLAY_INSTANCE, &patch("a.fxp")));
    assert_eq!(sink.prepared_chains(), vec![REVERB.to_string()]);
}

/// chain だけ違う行は、音色が同じでも準備し直す。戻るときは chain 付きの instance を使い回す。
#[test]
fn a_line_that_only_drops_the_chain_is_prepared_again() {
    let sink = FakeSink {
        two_banks: true,
        ..FakeSink::default()
    };
    let mut voice = voice();
    voice
        .prepare_line(&sink, &with_chain("a.fxp", REVERB))
        .unwrap();
    voice.play_line(&sink, &line(1));
    assert_eq!(sink.take()[0], Sent::Prepare(0, Some("a.fxp".to_string())));

    voice.prepare_line(&sink, &patch("a.fxp")).unwrap();
    voice.play_line(&sink, &line(1));
    assert_eq!(
        sink.take(),
        vec![
            Sent::StandbyPrepare(1, Some("a.fxp".to_string())),
            Sent::BeginTimeline,
            Sent::TimelineEvents(1),
        ]
    );
    assert_eq!(
        sink.prepared_chains(),
        vec![REVERB.to_string(), String::new()]
    );

    voice
        .prepare_line(&sink, &with_chain("a.fxp", REVERB))
        .unwrap();
    voice.play_line(&sink, &line(1));
    assert_eq!(
        sink.take(),
        vec![Sent::BeginTimeline, Sent::TimelineEvents(1)]
    );
    assert_eq!(sink.timeline_events().last().unwrap().instance_id, 0);
}
