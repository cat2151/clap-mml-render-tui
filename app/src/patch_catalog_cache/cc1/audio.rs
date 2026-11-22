use anyhow::{ensure, Context, Result};
use cmrt_core::{CoreConfig, PluginEntry, RealtimeRenderer};
use cmrt_tui_core::patch_catalog_cache::cc1::{Conditions, Observation};

use super::{
    pitch,
    worker::{Plugin, Request},
};

pub(super) const SAMPLE_RATE: usize = 48_000;
pub(super) const BLOCK: usize = 512;
const PREROLL_BLOCKS: usize = 10;
const DURATION_SECONDS: usize = 2;
const NOTE: u8 = 60;
const VELOCITY: u8 = 100;
const CC1_VALUES: [u8; 2] = [0, 127];

pub(super) fn observe(
    plugin: &Plugin,
    entry: &PluginEntry,
    request: Request,
) -> Result<Observation> {
    let cfg = CoreConfig {
        plugin_id: Some(plugin.id.clone()),
        patch_base: plugin.patch_base.clone(),
        patch_path: Some(request.patch_path),
        sample_rate: SAMPLE_RATE as f64,
        buffer_size: BLOCK,
        ..Default::default()
    };
    let low = render(&cfg, entry, CC1_VALUES[0])?;
    let high = render(&cfg, entry, CC1_VALUES[1])?;
    let (classification, frame_samples, trials) = pitch::compare([&low, &high]);
    Ok(Observation::Audio {
        plugin_version: plugin.version.clone(),
        routing_unresolved: request.routing_unresolved,
        conditions: Conditions {
            sample_rate_hz: SAMPLE_RATE,
            duration_seconds: DURATION_SECONDS,
            block_samples: BLOCK,
            preroll_blocks: PREROLL_BLOCKS,
            note_number: NOTE,
            velocity: VELOCITY,
            cc1_values: CC1_VALUES,
            fresh_instance_per_cc: true,
            analysis: "nsdf_pitch_comparison_v2".into(),
        },
        classification,
        frame_samples,
        trials,
    })
}

fn render(cfg: &CoreConfig, entry: &PluginEntry, cc: u8) -> Result<Vec<f32>> {
    let needed = SAMPLE_RATE * DURATION_SECONDS * 2;
    let mut renderer = RealtimeRenderer::new_for_offline(cfg, entry)
        .with_context(|| format!("CC1={cc}: 音色の初期化"))?;
    let mut process = |events: &[[u8; 3]]| {
        let block = renderer
            .render_live_chunk(events)
            .with_context(|| format!("CC1={cc}: ヘッドレス描画"))?;
        ensure!(
            block.len() == BLOCK * 2 && block.iter().all(|value| value.is_finite()),
            "CC1={cc}: 描画したステレオブロックが不正です"
        );
        Ok(block)
    };
    process(&[[0xb0, 1, cc]])?;
    for _ in 1..PREROLL_BLOCKS {
        process(&[])?;
    }
    let mut audio = Vec::with_capacity(needed.next_multiple_of(BLOCK * 2));
    audio.extend(process(&[[0x90, NOTE, VELOCITY]])?);
    while audio.len() < needed {
        audio.extend(process(&[])?);
    }
    process(&[[0x80, NOTE, 0]])?;
    audio.truncate(needed);
    Ok(audio)
}
