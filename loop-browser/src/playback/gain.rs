pub fn effective_track_gain(track: usize, track_volumes_db: &[i32], solo_tracks: &[bool]) -> f32 {
    let solo_mode_active = solo_tracks.iter().any(|&is_solo| is_solo);
    if solo_mode_active && !solo_tracks.get(track).copied().unwrap_or(false) {
        return 0.0;
    }
    cmrt_tui_core::mixer::volume_db_to_gain(track_volumes_db.get(track).copied().unwrap_or(0))
}
