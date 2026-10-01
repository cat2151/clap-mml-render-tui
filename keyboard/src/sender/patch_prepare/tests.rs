use super::*;

const CHAIN: &str = r#"[{"plugin":"x"}]"#;

#[test]
fn an_empty_chain_keeps_the_plain_prepare_and_probe() {
    assert_eq!(
        prepare_steps(
            PrepareTrigger::Patch {
                known_voicing: Some(PatchVoicing::Poly)
            },
            ""
        ),
        vec![PrepareStep::PrepareWithChain]
    );
    assert_eq!(
        prepare_steps(
            PrepareTrigger::Patch {
                known_voicing: None
            },
            ""
        ),
        vec![PrepareStep::ProbeVoicing]
    );
}

#[test]
fn a_known_voicing_sends_one_prepare_with_the_chain() {
    assert_eq!(
        prepare_steps(
            PrepareTrigger::Patch {
                known_voicing: Some(PatchVoicing::Mono)
            },
            CHAIN
        ),
        vec![PrepareStep::PrepareWithChain]
    );
}

#[test]
fn an_unknown_voicing_probes_then_puts_the_chain_back() {
    assert_eq!(
        prepare_steps(
            PrepareTrigger::Patch {
                known_voicing: None
            },
            CHAIN
        ),
        vec![PrepareStep::ProbeVoicing, PrepareStep::PrepareWithChain]
    );
}

#[test]
fn a_chain_change_sends_one_prepare_only_after_a_patch_is_loaded() {
    for chain in ["", CHAIN] {
        assert_eq!(
            prepare_steps(
                PrepareTrigger::EffectChain {
                    patch_prepared: true
                },
                chain
            ),
            vec![PrepareStep::PrepareWithChain]
        );
        assert!(prepare_steps(
            PrepareTrigger::EffectChain {
                patch_prepared: false
            },
            chain
        )
        .is_empty());
    }
}

#[test]
fn the_worker_resends_the_current_patch_when_only_the_chain_changes() {
    let mut state = LivePatchState::default();
    assert!(state.plan_effect_chain(CHAIN.to_string()).steps.is_empty());

    let plan = state.plan_patch(Some("Pads/Warm.fxp".to_string()), None, None);
    assert_eq!(plan.effect_chain, CHAIN);
    assert_eq!(
        plan.steps,
        vec![PrepareStep::ProbeVoicing, PrepareStep::PrepareWithChain]
    );
    state.finish_patch(true);

    assert_eq!(
        state.plan_effect_chain(String::new()),
        PreparePlan {
            patch: Some("Pads/Warm.fxp".to_string()),
            effect_chain: String::new(),
            steps: vec![PrepareStep::PrepareWithChain],
        }
    );
}

#[test]
fn a_patch_change_keeps_the_remembered_chain() {
    let mut state = LivePatchState::default();
    state.plan_patch(
        Some("A.fxp".to_string()),
        Some(PatchVoicing::Poly),
        Some(CHAIN.to_string()),
    );
    state.finish_patch(true);

    let plan = state.plan_patch(Some("B.fxp".to_string()), Some(PatchVoicing::Mono), None);
    assert_eq!(plan.patch.as_deref(), Some("B.fxp"));
    assert_eq!(plan.effect_chain, CHAIN);
    assert_eq!(plan.steps, vec![PrepareStep::PrepareWithChain]);
}

#[test]
fn a_failed_patch_prepare_holds_back_chain_only_sends() {
    let mut state = LivePatchState::default();
    state.plan_patch(Some("A.fxp".to_string()), Some(PatchVoicing::Poly), None);
    state.finish_patch(false);
    assert!(state.plan_effect_chain(CHAIN.to_string()).steps.is_empty());
}
