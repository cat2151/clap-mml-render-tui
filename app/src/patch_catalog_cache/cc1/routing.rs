use std::{collections::HashMap, fs, path::Path};

use anyhow::{bail, ensure, Context, Result};
use cmrt_core::{AudioPluginInfo, CoreConfig, PluginEntry, RealtimeRenderer};
use cmrt_tui_core::patch_catalog_cache::cc1::{Observation, RoutingClassification};

mod dexed;
mod surge;

const DEXED: &str = "com.digital-suburban.dexed";
const JC303: &str = "co.midilab.JC303";
const OBXF: &str = "org.surge-synth-team.OB-Xf";
const OBXF_VERSION: &str = "1.0.3.1223e6f";
const OBXF_REVISION: &str = "1223e6f7b209b9affa91331c26f39d4095a4088e";

pub(super) struct Inspector<'a> {
    version: &'a str,
    routing: Routing,
}

enum Routing {
    Obxf,
    Jc303,
    Dexed {
        cartridges: HashMap<String, Result<(), String>>,
        wheel: String,
    },
    Surge,
    Sforzando,
    Unresolved(String),
}

impl<'a> Inspector<'a> {
    pub(super) fn new(
        id: &str,
        version: &'a str,
        info: &AudioPluginInfo,
        entry: &PluginEntry,
    ) -> Self {
        let routing = match id {
            OBXF => verify_obxf_build(version, &info.plugin_path).map(|()| Routing::Obxf),
            JC303 if version == "0.12.3" => Ok(Routing::Jc303),
            DEXED if version == "1.0.1" => dexed_wheel(info, entry).map(|wheel| Routing::Dexed {
                cartridges: HashMap::new(),
                wheel,
            }),
            "org.surge-synth-team.surge-xt" if version.starts_with("1.") => Ok(Routing::Surge),
            "com.Plogue Art et Technologie, Inc.sforzando" => Ok(Routing::Sforzando),
            _ => Err(anyhow::anyhow!(
                "{id} {version}: verified CC1 routing decoder unavailable"
            )),
        };
        if let Err(error) = &routing {
            super::report_progress(format!(
                "CC1 {id} {version}: 接続判定を準備できません: {error:#}"
            ));
        }
        Self {
            version,
            routing: routing.unwrap_or_else(|error| Routing::Unresolved(format!("{error:#}"))),
        }
    }

    pub(super) fn observe(&mut self, path: &str) -> Result<Observation> {
        use RoutingClassification::*;
        let (version, classification, evidence) = match &mut self.routing {
            Routing::Obxf => {
                ensure!(Path::new(path).is_file(), "OB-Xf preset is missing: {path}");
                (
                    OBXF_VERSION,
                    Vibrato,
                    format!("OB-Xf {OBXF_REVISION}: CC1 -> processModWheelSmoothed -> vibratoAmount -> vibratoLFO -> oscillator pitch (instrument-wide MIDI routing)"),
                )
            }
            Routing::Jc303 => {
                ensure!(Path::new(path).is_file(), "JC303 preset is missing: {path}");
                (
                    self.version,
                    Unconnected,
                    "JC303 abac3b353a8eb505a45cbc884112da9e8b9b5888: processBlock handles notes and All Notes Off. CC1 is unhandled for every preset.".into(),
                )
            }
            Routing::Dexed { cartridges, wheel } => {
                let program = cmrt_core::parse_cartridge_patch_path(path)?;
                let validation = cartridges
                    .entry(program.cartridge_path.clone())
                    .or_insert_with(|| {
                        validate_cartridge(&program.cartridge_path)
                            .map_err(|error| format!("{error:#}"))
                    });
                if let Err(error) = validation {
                    bail!("{error}");
                }
                (
                    self.version,
                    Unconnected,
                    format!("Dexed bce5deee7c41bf5515b806d0b7de8b5c0bb49467: initialized CLAP state wheelMod={wheel:?} (range, pitch, amp, EG), no CC1 MIDI mapping. Program {} in the validated cartridge is loaded by voice SysEx, which has no controller configuration fields.", program.program_index),
                )
            }
            Routing::Surge => {
                let (classification, evidence) = surge::observe(Path::new(path))?;
                (self.version, classification, evidence)
            }
            Routing::Sforzando if cmrt_core::is_sfz_patch_path(path) => {
                let files = cmrt_core::sfz_files_without_cc1(Path::new(path))?;
                (
                    self.version,
                    Unconnected,
                    format!("SFZ program and includes ({files} files, macros expanded): no CC1 reference or script. Scope: SFZ-defined routing."),
                )
            }
            Routing::Sforzando => {
                bail!("CC1 routing for an ARIA instrument is not available from SFZ opcodes")
            }
            Routing::Unresolved(reason) => bail!("{reason}"),
        };
        Ok(Observation::Routing {
            plugin_version: version.into(),
            classification,
            evidence,
        })
    }
}

fn validate_cartridge(path: &str) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("cannot read DX7 cartridge {path}"))?;
    cmrt_core::parse_dx7_cartridge(bytes)
        .with_context(|| format!("invalid DX7 cartridge {path}"))?;
    Ok(())
}

fn dexed_wheel(info: &AudioPluginInfo, entry: &PluginEntry) -> Result<String> {
    let config = CoreConfig {
        plugin_id: info.plugin_id.clone(),
        sample_rate: super::audio::SAMPLE_RATE as f64,
        buffer_size: super::audio::BLOCK,
        ..Default::default()
    };
    let renderer = RealtimeRenderer::new_for_offline(&config, entry)
        .context("Dexed initialized state could not be prepared")?;
    let xml = cmrt_core::surge_fx_preset::juce_xml::decode(
        renderer
            .initial_state()
            .context("Dexed state is unavailable")?,
    )?;
    dexed::wheel_without_cc1(&xml)
}

fn verify_obxf_build(descriptor_version: &str, plugin_path: &str) -> Result<()> {
    if descriptor_version == OBXF_VERSION {
        return Ok(());
    }
    // The investigated Windows build advertises 1.0.3 in the descriptor and
    // embeds the full build identifier it logs on instantiation.
    ensure!(
        descriptor_version == "1.0.3" && Path::new(plugin_path).is_file(),
        "OB-Xf CC1 routing has not been verified for {descriptor_version}"
    );
    let binary = fs::read(plugin_path).context("cannot read OB-Xf build identifier")?;
    let marker = b"1.0.3.1223e6f\0";
    ensure!(
        binary.windows(marker.len()).any(|window| window == marker),
        "OB-Xf CC1 routing has not been verified for this {descriptor_version} build"
    );
    Ok(())
}
