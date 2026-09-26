//! 保存済み daily project から、セルごとの render 内容と cache WAV の置き場を並べる。
//!
//! DAW 画面を起動せずに「cache がどの MML から作られたはずか」を取り出すための口。
//! MML は画面と同じ [`build_cell_mml_from_data`] で組むので、ここで得た MML を
//! render した結果は、正しく作られた cache WAV と一致するはずのもの。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cmrt_history::daw_cache_mml_hash;

use crate::cache::cache_wav_path;
use crate::daily::{daily_current_path, load_daily_recovery};
use crate::mml::{build_cell_mml_from_data, cell_has_content, measure_duration_samples_from_data};
use crate::project::project_snapshot_for_recovery;
use crate::tracks::{track_display_number, track_renders_audio};
use crate::{WorkspaceKind, FIRST_PLAYABLE_TRACK};

/// render 対象セル 1 つぶん。
#[derive(Clone, Debug)]
pub struct DawCellRenderPlan {
    /// グリッドの行 index。cache WAV のファイル名 `track{行}_meas{小節}.wav` に使われる番号。
    pub grid_row: usize,
    /// 画面・mixer・project file が使う track 番号。
    pub display_track: usize,
    /// 1 始まりの小節番号。
    pub measure: usize,
    pub mml: String,
    pub mml_hash: u64,
    /// daily recovery に記録された、cache を作ったときの MML hash。記録が無ければ `None`。
    pub saved_mml_hash: Option<u64>,
    pub cache_wav: Option<PathBuf>,
}

/// daily project 全体の render 計画。
#[derive(Clone, Debug)]
pub struct DawDailyRenderPlan {
    pub recovery_path: PathBuf,
    pub page_date: String,
    /// 1 小節のサンプル数（ステレオ interleave）。
    pub measure_samples: usize,
    pub cells: Vec<DawCellRenderPlan>,
}

/// `config_app_dir` の daily recovery を読み、音を持つ全セルの render 計画を返す。
pub fn daily_render_plan(config_app_dir: &Path, sample_rate: f64) -> Result<DawDailyRenderPlan> {
    let recovery_path = daily_current_path(config_app_dir);
    let recovery = load_daily_recovery(&recovery_path)?
        .with_context(|| format!("Daily recovery がありません: {}", recovery_path.display()))?;
    let snapshot = project_snapshot_for_recovery(&recovery.project_file)?;
    let data = &snapshot.data;

    let mut cells = Vec::new();
    for grid_row in FIRST_PLAYABLE_TRACK..snapshot.tracks {
        if !track_renders_audio(grid_row) {
            continue;
        }
        for measure in 1..=snapshot.measures {
            if !cell_has_content(data, grid_row, measure) {
                continue;
            }
            let mml = build_cell_mml_from_data(data, snapshot.measures, grid_row, measure);
            let saved_mml_hash = recovery
                .cached_measures
                .iter()
                .find(|entry| entry.track == grid_row && entry.measure == measure)
                .map(|entry| entry.mml_hash);
            cells.push(DawCellRenderPlan {
                grid_row,
                display_track: track_display_number(grid_row),
                measure,
                mml_hash: daw_cache_mml_hash(&mml),
                mml,
                saved_mml_hash,
                cache_wav: cache_wav_path(WorkspaceKind::Daily, grid_row, measure),
            });
        }
    }

    Ok(DawDailyRenderPlan {
        recovery_path,
        page_date: recovery.page_date,
        measure_samples: measure_duration_samples_from_data(data, snapshot.measures, sample_rate),
        cells,
    })
}
