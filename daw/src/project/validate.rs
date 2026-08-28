//! project file の検証と、グリッドへの読み込み。
//!
//! 「壊れた project file をそのまま app へ流し込まない」ための関門をここへ閉じる。
//! DTO 定義と app 側の入出力は親モジュールにある。

use anyhow::{bail, Context, Result};

use super::{
    cell_role, grid_row_from_saved_track, grid_track_count_from_saved, track_role, DawProjectCell,
    DawProjectFile, DawProjectSnapshot, CHORD_TRACK, FIRST_SAVED_PLAYABLE_TRACK, MIXER_MAX_DB,
    MIXER_MIN_DB, PROJECT_FORMAT, PROJECT_FORMAT_VERSION,
};

fn try_empty_grid(tracks: usize, measures: usize) -> Result<Vec<Vec<String>>> {
    let columns = measures
        .checked_add(1)
        .context("playable_measure_count が大きすぎます")?;
    tracks
        .checked_mul(columns)
        .context("project grid が大きすぎます")?;

    let mut data = Vec::new();
    data.try_reserve_exact(tracks)
        .context("project track 用メモリを確保できません")?;
    for _ in 0..tracks {
        let mut row = Vec::new();
        row.try_reserve_exact(columns)
            .context("project measure 用メモリを確保できません")?;
        row.resize_with(columns, String::new);
        data.push(row);
    }
    Ok(data)
}

/// 1 行ぶんの `non_empty_cells` の並びと role を検証する。
///
/// `label` はエラーメッセージ用（`track3` / `chord track`）。
fn validate_project_cells(
    cells: &[DawProjectCell],
    label: &str,
    playable_measure_count: usize,
) -> Result<()> {
    let mut previous_measure = None;
    for cell in cells {
        if cell.measure_index > playable_measure_count {
            bail!(
                "{label} の measure_index が範囲外です: {}",
                cell.measure_index
            );
        }
        if previous_measure.is_some_and(|previous| cell.measure_index <= previous) {
            bail!("{label} の non_empty_cells は measure_index 順に重複なく並べる必要があります");
        }
        if cell.role != cell_role(cell.measure_index) {
            bail!(
                "{label} measure{} の role が index と一致しません",
                cell.measure_index
            );
        }
        if cell.mml.trim().is_empty() {
            bail!(
                "{label} measure{} は non_empty_cells 内で空にできません",
                cell.measure_index
            );
        }
        previous_measure = Some(cell.measure_index);
    }
    Ok(())
}

pub(super) fn validate_project_file(file: &DawProjectFile) -> Result<DawProjectSnapshot> {
    if file.format != PROJECT_FORMAT {
        bail!(
            "project format が違います: expected={PROJECT_FORMAT}, actual={}",
            file.format
        );
    }
    if file.format_version != PROJECT_FORMAT_VERSION {
        bail!(
            "未対応の project format_version です: {}",
            file.format_version
        );
    }

    let project = &file.project;
    if project.track_count <= FIRST_SAVED_PLAYABLE_TRACK {
        bail!("track_count は2以上である必要があります");
    }
    if project.playable_measure_count == 0 {
        bail!("playable_measure_count は1以上である必要があります");
    }
    if project.tracks.len() != project.track_count {
        bail!(
            "tracks の要素数が track_count と一致しません: {} != {}",
            project.tracks.len(),
            project.track_count
        );
    }

    let grid_track_count = grid_track_count_from_saved(project.track_count);
    let mut data = try_empty_grid(grid_track_count, project.playable_measure_count)?;
    let mut track_volumes_db = Vec::new();
    track_volumes_db
        .try_reserve_exact(grid_track_count)
        .context("track volume 用メモリを確保できません")?;
    track_volumes_db.resize(grid_track_count, 0);

    for (expected_track_index, track) in project.tracks.iter().enumerate() {
        if track.track_index != expected_track_index {
            bail!(
                "tracks は track_index 順に重複なく並べる必要があります: expected={}, actual={}",
                expected_track_index,
                track.track_index
            );
        }
        if track.role != track_role(track.track_index) {
            bail!("track{} の role が index と一致しません", track.track_index);
        }
        if track.track_index == 0 && track.volume_db != 0 {
            bail!("global header track の volume_db は0である必要があります");
        }
        if !(MIXER_MIN_DB..=MIXER_MAX_DB).contains(&track.volume_db) {
            bail!(
                "track{} の volume_db が範囲外です: {}",
                track.track_index,
                track.volume_db
            );
        }
        let row_index = grid_row_from_saved_track(track.track_index);
        track_volumes_db[row_index] = track.volume_db;

        validate_project_cells(
            &track.non_empty_cells,
            &format!("track{}", track.track_index),
            project.playable_measure_count,
        )?;
        for cell in &track.non_empty_cells {
            data[row_index][cell.measure_index] = cell.mml.clone();
        }
    }

    if let Some(chord_track) = &project.chord_track {
        validate_project_cells(
            &chord_track.non_empty_cells,
            "chord track",
            project.playable_measure_count,
        )?;
        for cell in &chord_track.non_empty_cells {
            data[CHORD_TRACK][cell.measure_index] = cell.mml.clone();
        }
    }

    Ok(DawProjectSnapshot {
        data,
        track_volumes_db,
        tracks: grid_track_count,
        measures: project.playable_measure_count,
    })
}

pub(crate) fn validate_project_file_for_recovery(file: &DawProjectFile) -> Result<()> {
    validate_project_file(file).map(|_| ())
}

pub(crate) fn project_snapshot_for_recovery(file: &DawProjectFile) -> Result<DawProjectSnapshot> {
    validate_project_file(file)
}
