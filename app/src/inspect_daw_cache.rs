//! daily DAW の全セルを画面と同じ MML で render し直し、今ある cache WAV と並べて print する。
//!
//! cache WAV だけを見ても「render が悪いのか、cache が古い/壊れているのか」は分からない。
//! 同じ MML の新しい render を対照に置き、音が鳴り終わる位置と包絡を 1 行ずつ比べる。

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use cmrt_daw::{daily_render_plan, DawCellRenderPlan};
use cmrt_offline_render::OfflineRenderer;
use cmrt_runtime::Config;

/// `cmrt inspect-daw-cache` の引数。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct InspectDawCacheRequest {
    /// 新しく render した WAV の書き出し先。省略時は書かない。
    pub out_dir: Option<PathBuf>,
    /// 「cache が新しい render より早く鳴り止む」と判定したセルの cache WAV を消す。
    /// 消したセルは、次に DAW を開いたとき render し直される。
    pub delete_broken: bool,
}

/// 包絡 1 本の桁数。小節ぶんをこの数に等分する。
const ENVELOPE_COLUMNS: usize = 32;

/// ピーク窓からこの比まで下がったら「鳴っていない」とみなす（-40dB）。
const SOUNDING_RATIO: f64 = 0.01;

/// 新しい render に比べて、cache の鳴り終わりがこの比より早ければ途切れとみなす。
const CUT_RATIO: f64 = 0.8;

pub fn run(cfg: &Config, request: &InspectDawCacheRequest) -> Result<()> {
    let config_app_dir =
        cmrt_runtime::config_app_dir().context("config の置き場を特定できません")?;
    let plan = daily_render_plan(&config_app_dir, cfg.sample_rate)?;
    if let Some(dir) = &request.out_dir {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("WAV の書き出し先を作れませんでした: {}", dir.display()))?;
    }
    let renderer = OfflineRenderer::new(std::sync::Arc::new(cfg.clone()));
    let measure_frames = plan.measure_samples / 2;
    let sample_rate = cfg.sample_rate as u32;

    println!("[inspect-daw-cache]");
    println!("  recovery      : {}", plan.recovery_path.display());
    println!("  page_date     : {}", plan.page_date);
    println!(
        "  measure       : {} frames ({} ms)",
        measure_frames,
        frames_to_ms(measure_frames, sample_rate)
    );
    println!("  cells         : {}", plan.cells.len());
    println!();

    let mut problems = Vec::new();
    for cell in &plan.cells {
        let fresh = renderer
            .render_phrase(&cell.mml)
            .with_context(|| format!("オフラインレンダリングに失敗しました: mml={}", cell.mml))?
            .samples;
        if let Some(dir) = &request.out_dir {
            let path = dir.join(format!("track{}_meas{}.wav", cell.grid_row, cell.measure));
            cmrt_core::write_wav(&fresh, sample_rate, &path)
                .with_context(|| format!("WAV を書けませんでした: {}", path.display()))?;
        }
        let cached = cell
            .cache_wav
            .as_deref()
            .filter(|path| path.exists())
            .map(cmrt_tui_core::wav_io::load_wav_samples)
            .transpose()?;
        let verdict = print_cell(cell, &fresh, cached.as_deref(), measure_frames, sample_rate);
        if verdict != CellVerdict::Ok {
            problems.push((cell, verdict));
        }
    }

    println!("[まとめ]");
    println!(
        "  問題のあるセル: {} / {}",
        problems.len(),
        plan.cells.len()
    );
    for (cell, verdict) in &problems {
        println!(
            "    track{} meas{}: {}",
            cell.display_track,
            cell.measure,
            verdict.label()
        );
    }
    if request.delete_broken {
        delete_cut_caches(&problems)?;
    }
    Ok(())
}

fn delete_cut_caches(problems: &[(&DawCellRenderPlan, CellVerdict)]) -> Result<()> {
    let targets: Vec<&Path> = problems
        .iter()
        .filter(|(_, verdict)| *verdict == CellVerdict::CacheCut)
        .filter_map(|(cell, _)| cell.cache_wav.as_deref())
        .collect();
    println!();
    println!("[--delete-broken]");
    println!("  消す cache WAV: {}", targets.len());
    for path in targets {
        std::fs::remove_file(path)
            .with_context(|| format!("cache WAV を消せませんでした: {}", path.display()))?;
        println!("    消した: {}", path.display());
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellVerdict {
    Ok,
    CacheMissing,
    /// cache を作った MML と今の MML が違う。
    StaleHash,
    /// cache の音が、同じ MML の新しい render より早く鳴り止む。
    CacheCut,
}

impl CellVerdict {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::CacheMissing => "cache WAV が無い",
            Self::StaleHash => "cache を作った MML と今の MML が違う",
            Self::CacheCut => "cache が新しい render より早く鳴り止む",
        }
    }
}

fn print_cell(
    cell: &DawCellRenderPlan,
    fresh: &[f32],
    cached: Option<&[f32]>,
    measure_frames: usize,
    sample_rate: u32,
) -> CellVerdict {
    let hash_state = match cell.saved_mml_hash {
        Some(saved) if saved == cell.mml_hash => "match",
        Some(_) => "MISMATCH",
        None => "none",
    };
    println!(
        "track{} (row{}) meas{}  hash={:016x} saved={}",
        cell.display_track, cell.grid_row, cell.measure, cell.mml_hash, hash_state
    );
    println!("  mml   : {}", cell.mml);

    // 包絡は両方を同じ基準で正規化する（片方だけ小さいことも見えるように）。
    let fresh_env = window_rms(fresh, measure_frames);
    let cached_env = cached.map(|samples| window_rms(samples, measure_frames));
    let scale = fresh_env
        .iter()
        .chain(cached_env.iter().flatten())
        .fold(0.0_f64, |max, value| max.max(*value));

    let fresh_end = sounding_end_frames(fresh);
    println!(
        "  fresh : |{}| sounding_until={}ms",
        envelope_digits(&fresh_env, scale),
        frames_to_ms(fresh_end, sample_rate)
    );
    let Some(cached) = cached else {
        println!("  cache : (なし)");
        println!();
        return CellVerdict::CacheMissing;
    };
    let cached_end = sounding_end_frames(cached);
    println!(
        "  cache : |{}| sounding_until={}ms",
        envelope_digits(cached_env.as_deref().unwrap_or_default(), scale),
        frames_to_ms(cached_end, sample_rate)
    );
    let verdict = classify(hash_state == "MISMATCH", fresh_end, cached_end);
    println!("  => {}", verdict.label());
    println!();
    verdict
}

pub(crate) fn classify(hash_mismatch: bool, fresh_end: usize, cached_end: usize) -> CellVerdict {
    if hash_mismatch {
        CellVerdict::StaleHash
    } else if (cached_end as f64) < fresh_end as f64 * CUT_RATIO {
        CellVerdict::CacheCut
    } else {
        CellVerdict::Ok
    }
}

/// 小節を [`ENVELOPE_COLUMNS`] 等分した窓ごとの RMS（左 ch）。小節より後ろの余韻は見ない。
pub(crate) fn window_rms(samples: &[f32], measure_frames: usize) -> Vec<f64> {
    let frames = samples.len() / 2;
    (0..ENVELOPE_COLUMNS)
        .map(|column| {
            let start = measure_frames * column / ENVELOPE_COLUMNS;
            let end = (measure_frames * (column + 1) / ENVELOPE_COLUMNS).min(frames);
            if start >= end {
                return 0.0;
            }
            let sum: f64 = (start..end)
                .map(|frame| f64::from(samples[frame * 2]).powi(2))
                .sum();
            (sum / (end - start) as f64).sqrt()
        })
        .collect()
}

pub(crate) fn envelope_digits(envelope: &[f64], scale: f64) -> String {
    envelope
        .iter()
        .map(|value| {
            if scale <= 0.0 {
                return '0';
            }
            let digit = (value / scale * 9.99).floor().clamp(0.0, 9.0) as u32;
            char::from_digit(digit, 10).unwrap_or('0')
        })
        .collect()
}

/// 最後に「ピークから -40dB 以内」の音があった位置（frame）。無音なら 0。
pub(crate) fn sounding_end_frames(samples: &[f32]) -> usize {
    let peak = samples.iter().fold(0.0_f32, |peak, s| peak.max(s.abs()));
    if peak <= 0.0 {
        return 0;
    }
    let threshold = peak * SOUNDING_RATIO as f32;
    samples
        .iter()
        .rposition(|sample| sample.abs() >= threshold)
        .map_or(0, |index| index / 2 + 1)
}

fn frames_to_ms(frames: usize, sample_rate: u32) -> u64 {
    match sample_rate {
        0 => 0,
        rate => frames as u64 * 1000 / u64::from(rate),
    }
}

#[cfg(test)]
mod tests;
