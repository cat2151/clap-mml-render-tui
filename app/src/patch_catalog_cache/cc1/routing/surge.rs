use std::{collections::BTreeSet, path::Path};

use anyhow::{ensure, Context, Result};
use cmrt_tui_core::patch_catalog_cache::cc1::RoutingClassification;
use xmltree::{Element, XMLNode};

const PLUGIN_ID: &str = "org.surge-synth-team.surge-xt";

pub(super) fn observe(path: &Path) -> Result<(RoutingClassification, String)> {
    let path = cmrt_core::lexical_absolute(path)?;
    let state = cmrt_core::prepare_clap_patch_state(PLUGIN_ID, &path)?;
    // The validated sub3 header gives the XML length before the wavetables.
    let size = u32::from_le_bytes(state[4..8].try_into().unwrap()) as usize;
    let xml = std::str::from_utf8(&state[32..32 + size])?.trim_end_matches('\0');
    inspect(&Element::parse(xml.as_bytes())?)
}

fn inspect(preset: &Element) -> Result<(RoutingClassification, String)> {
    use RoutingClassification::*;
    let revision = preset
        .attributes
        .get("revision")
        .context("Surge FXP has no revision")?;
    ensure!(
        preset.name == "patch" && (9..=26).contains(&revision.parse::<u8>()?),
        "unverified Surge FXP revision {revision}"
    );
    ensure!(
        preset
            .get_child("formulae")
            .is_none_or(|formulae| elements(formulae).next().is_none()),
        "Surge FXP contains Formula scripts"
    );
    ensure!(
        preset.get_child("dawExtraState").is_none_or(|state| {
            state.get_child("midictrl_map").is_none()
                && state.get_child("customcontrol_map").is_none()
        }),
        "Surge FXP contains an embedded MIDI controller map"
    );
    let parameters = preset
        .get_child("parameters")
        .context("Surge FXP has no parameter list")?;
    let mut targets = BTreeSet::new();
    for parameter in elements(parameters) {
        for route in elements(parameter).filter(|element| element.name == "modrouting") {
            if route
                .attributes
                .get("source")
                .context("Surge modulation has no source")?
                .parse::<i32>()?
                != 6
            {
                continue;
            }
            let depth = route
                .attributes
                .get("depth")
                .context("Surge modulation has no depth")?
                .parse::<f64>()?;
            ensure!(depth.is_finite(), "Surge modulation depth is not finite");
            let muted = route
                .attributes
                .get("muted")
                .map(|value| value.parse::<u8>())
                .transpose()?
                .unwrap_or(0);
            if depth != 0.0 && muted == 0 {
                targets.insert(parameter.name.as_str());
            }
        }
    }
    let classification = if targets.iter().any(|target| is_direct_pitch(target)) {
        FixedPitchChange
    } else if targets.is_empty() {
        Unconnected
    } else {
        ensure!(
            targets.iter().all(|target| is_filter(target)),
            "Surge FXP source=6 (CC1) has unclassified targets: {targets:?}"
        );
        NonPitchRouting
    };
    let evidence = format!(
        "Surge FXP revision {revision}: modulation source=6 (Mod Wheel), active destinations {targets:?}; no Formula script or embedded controller map. Scope: preset routing."
    );
    Ok((classification, evidence))
}

fn elements(element: &Element) -> impl Iterator<Item = &Element> {
    element.children.iter().filter_map(XMLNode::as_element)
}

fn scene_parameter(name: &str) -> &str {
    name.strip_prefix("a_")
        .or_else(|| name.strip_prefix("b_"))
        .unwrap_or(name)
}

fn is_direct_pitch(name: &str) -> bool {
    matches!(
        scene_parameter(name),
        "pitch" | "osc1_pitch" | "osc2_pitch" | "osc3_pitch"
    )
}

fn is_filter(name: &str) -> bool {
    matches!(
        scene_parameter(name),
        "filter1_cutoff" | "filter2_cutoff" | "filter1_resonance" | "filter2_resonance"
    )
}

#[cfg(test)]
mod tests;
