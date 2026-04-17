use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::DawHttpState;

pub(in crate::daw::http_server) fn get_snapshot_mml(
    state: &DawHttpState,
    track: usize,
    measure: usize,
) -> Result<String, (u16, String)> {
    if state.grid_snapshot.is_empty() {
        return Err((503, "DAW データの準備中です\n".to_string()));
    }
    state
        .grid_snapshot
        .get(track)
        .and_then(|row| row.get(measure))
        .cloned()
        .ok_or_else(|| {
            (
                404,
                format!(
                    "指定された track/measure は範囲外です: track={track}, measure={measure}\n"
                ),
            )
        })
}

pub(in crate::daw::http_server) fn get_snapshot_mmls(
    state: &DawHttpState,
) -> Result<Vec<Vec<String>>, (u16, String)> {
    if state.grid_snapshot.is_empty() {
        return Err((503, "DAW データの準備中です\n".to_string()));
    }
    Ok(state.grid_snapshot.clone())
}

pub(in crate::daw::http_server) fn parse_get_mml_query(
    url: &str,
) -> Result<(usize, usize), (u16, String)> {
    let Some((_, query)) = url.split_once('?') else {
        return Err((400, "track と measure を指定してください\n".to_string()));
    };
    let mut track = None;
    let mut measure = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "track" => track = Some(parse_query_usize("track", value)?),
            "measure" | "meas" => measure = Some(parse_query_usize("measure", value)?),
            _ => {}
        }
    }
    match (track, measure) {
        (Some(track), Some(measure)) => Ok((track, measure)),
        _ => Err((400, "track と measure を指定してください\n".to_string())),
    }
}

fn parse_query_usize(name: &str, value: &str) -> Result<usize, (u16, String)> {
    if value.is_empty() {
        return Err((400, format!("{name} を指定してください\n")));
    }
    value
        .parse::<usize>()
        .map_err(|_| (400, format!("{name} は 0 以上の整数を指定してください\n")))
}

/// Builds an opaque ETag for the current DAW snapshot body.
///
/// This uses `DefaultHasher`, so the exact value is only intended for
/// same-process conditional GETs and may change across Rust versions or
/// server restarts. That is acceptable here because clients only need a
/// best-effort validator for `If-None-Match` on `/mmls`.
pub(in crate::daw::http_server) fn snapshot_mmls_etag(tracks: &[Vec<String>]) -> String {
    let mut hasher = DefaultHasher::new();
    tracks.hash(&mut hasher);
    format!("\"{:016x}\"", hasher.finish())
}

pub(in crate::daw::http_server) fn if_none_match_matches(header_value: &str, etag: &str) -> bool {
    header_value
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == "*" || normalize_etag(candidate) == normalize_etag(etag))
}

fn normalize_etag(tag: &str) -> &str {
    let trimmed = tag.trim();
    let without_weak_prefix = trimmed.strip_prefix("W/").unwrap_or(trimmed).trim();
    without_weak_prefix
        .strip_prefix('"')
        .and_then(|stripped| stripped.strip_suffix('"'))
        .unwrap_or(without_weak_prefix)
}
