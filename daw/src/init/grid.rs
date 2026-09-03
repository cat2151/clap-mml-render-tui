use crate::{CellCache, TrackRerenderBatch, DEFAULT_TRACK0_MML, MEASURES, TRACKS};

pub(super) struct DawGridBuffers {
    pub(super) tracks: usize,
    pub(super) measures: usize,
    pub(super) data: Vec<Vec<String>>,
    pub(super) cache: Vec<Vec<CellCache>>,
    pub(super) track_rerender_batches: Vec<Option<TrackRerenderBatch>>,
    pub(super) play_measure_mmls: Vec<String>,
    pub(super) play_measure_track_mmls: Vec<Vec<String>>,
    pub(super) solo_tracks: Vec<bool>,
    pub(super) track_volumes_db: Vec<i32>,
}

fn try_build_string_row(len: usize) -> Option<Vec<String>> {
    let mut row = Vec::new();
    row.try_reserve_exact(len).ok()?;
    row.resize_with(len, String::new);
    Some(row)
}

fn try_build_cache_row(len: usize) -> Option<Vec<CellCache>> {
    let mut row = Vec::new();
    row.try_reserve_exact(len).ok()?;
    row.resize_with(len, CellCache::empty);
    Some(row)
}

fn try_build_string_grid(rows: usize, cols: usize) -> Option<Vec<Vec<String>>> {
    let mut grid = Vec::new();
    grid.try_reserve_exact(rows).ok()?;
    for _ in 0..rows {
        grid.push(try_build_string_row(cols)?);
    }
    Some(grid)
}

fn try_build_cache_grid(rows: usize, cols: usize) -> Option<Vec<Vec<CellCache>>> {
    let mut grid = Vec::new();
    grid.try_reserve_exact(rows).ok()?;
    for _ in 0..rows {
        grid.push(try_build_cache_row(cols)?);
    }
    Some(grid)
}

fn try_build_none_vec<T>(len: usize) -> Option<Vec<Option<T>>> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).ok()?;
    values.resize_with(len, || None);
    Some(values)
}

fn try_build_default_vec<T: Clone>(len: usize, value: T) -> Option<Vec<T>> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).ok()?;
    values.resize(len, value);
    Some(values)
}

pub(super) fn try_build_grid_buffers(tracks: usize, measures: usize) -> Option<DawGridBuffers> {
    let columns = measures.checked_add(1)?;
    let _data_cells = tracks.checked_mul(columns)?;
    let _play_measure_cells = measures.checked_mul(tracks)?;

    let mut data = try_build_string_grid(tracks, columns)?;
    data[0][0] = DEFAULT_TRACK0_MML.to_string();

    Some(DawGridBuffers {
        tracks,
        measures,
        data,
        cache: try_build_cache_grid(tracks, columns)?,
        track_rerender_batches: try_build_none_vec(tracks)?,
        play_measure_mmls: try_build_string_row(measures)?,
        play_measure_track_mmls: try_build_string_grid(measures, tracks)?,
        solo_tracks: try_build_default_vec(tracks, false)?,
        track_volumes_db: try_build_default_vec(tracks, 0)?,
    })
}

pub(super) fn build_grid_buffers_or_default(
    saved_grid_dimensions: Option<(usize, usize)>,
) -> DawGridBuffers {
    let (requested_tracks, requested_measures) = saved_grid_dimensions
        .map(|(tracks, measures)| (TRACKS.max(tracks), MEASURES.max(measures)))
        .unwrap_or((TRACKS, MEASURES));

    if let Some(buffers) = try_build_grid_buffers(requested_tracks, requested_measures) {
        return buffers;
    }

    // DAW アプリ本体はまだ未構築で、TUI 中の stderr は実画面を壊すため永続ログへ退避する。
    crate::log_line(&format!(
        "DAW セッションのサイズが大きすぎるか破損しているため、デフォルトサイズ {}x{} にフォールバックします。",
        TRACKS, MEASURES
    ));
    try_build_grid_buffers(TRACKS, MEASURES)
        .expect("default DAW grid should be allocatable in supported environments")
}
