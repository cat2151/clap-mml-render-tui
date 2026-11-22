use anyhow::{ensure, Context, Result};
use xmltree::{Element, XMLNode};

pub(super) fn wheel_without_cc1(xml: &str) -> Result<String> {
    // JUCE encodes binary attributes as base64:<name> without binding the
    // prefix. Bind it for namespace-aware parsers before inspecting the state.
    let xml = xml.replacen(
        "<dexedState ",
        "<dexedState xmlns:base64=\"urn:juce:base64\" ",
        1,
    );
    let state = Element::parse(xml.as_bytes())?;
    ensure!(state.name == "dexedState", "unexpected Dexed state root");
    let wheel = state
        .attributes
        .get("wheelMod")
        .context("Dexed wheelMod is missing")?;
    ensure!(
        wheel.split_whitespace().eq(["0", "0", "0", "0"]),
        "Dexed initialized wheelMod={wheel:?} requires audio verification"
    );
    let mappings = state
        .get_child("midiCC")
        .context("Dexed MIDI mapping state is missing")?;
    for mapping in mappings.children.iter().filter_map(XMLNode::as_element) {
        let cc = mapping
            .attributes
            .get("cc")
            .context("Dexed MIDI mapping has no CC number")?
            .parse::<u16>()?;
        // Dexed packs the channel into the high byte of the controller number.
        ensure!(cc & 0xff != 1, "Dexed has a learned MIDI CC1 mapping");
    }
    Ok(wheel.clone())
}

#[cfg(test)]
mod tests;
