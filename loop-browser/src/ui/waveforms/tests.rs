use super::*;

#[test]
fn one_shot_uses_only_its_real_duration_on_the_span_timeline() {
    assert_eq!(
        waveform_output_columns(LoopWavKind::OneShot, 4.785, 4, 2.0, 64),
        39
    );
}

#[test]
fn one_shot_ending_on_its_boundary_uses_the_whole_span() {
    assert_eq!(
        waveform_output_columns(LoopWavKind::OneShot, 4.0, 2, 2.0, 32),
        32
    );
}

#[test]
fn loop_still_uses_the_whole_stretched_span() {
    assert_eq!(
        waveform_output_columns(LoopWavKind::Loop, 4.785, 4, 2.0, 64),
        64
    );
}

#[test]
fn columns_after_a_one_shot_are_silent_instead_of_stretched_waveform() {
    let waveform = LoopWaveform {
        rms_db_tenths: vec![-60; 32],
        spectral_flux: vec![0; 32],
        centroid_motion_millioctaves: 0,
    };
    let waveform_len = waveform_output_columns(LoopWavKind::OneShot, 4.785, 4, 2.0, 64);

    assert_ne!(
        render_timeline_column(
            &waveform,
            WaveformDisplayScale::default(),
            waveform_len - 1,
            waveform_len
        )
        .content,
        "·"
    );
    assert_eq!(
        render_timeline_column(
            &waveform,
            WaveformDisplayScale::default(),
            waveform_len,
            waveform_len
        )
        .content,
        "·"
    );
}

#[test]
fn motion_thresholds_select_blue_green_and_orange() {
    assert_eq!(theme_hue(0.0), BLUE_HUE);
    assert_eq!(theme_hue(ONE_SEMITONE_OCTAVES), GREEN_HUE);
    assert_eq!(theme_hue(SIX_SEMITONES_OCTAVES), ORANGE_HUE);
}

#[test]
fn aggregate_rms_uses_mean_energy_instead_of_peak() {
    assert_eq!(aggregate_rms(&[-200, -200]), -200);
    assert_eq!(aggregate_rms(&[SILENCE_DB_TENTHS; 2]), SILENCE_DB_TENTHS);
    assert!(aggregate_rms(&[-100, -300]) < -100);
}

#[test]
fn hsl_output_uses_requested_dominant_theme_channel() {
    let Color::Rgb(orange_r, orange_g, orange_b) = hsl_to_rgb(ORANGE_HUE, 0.8, 0.5) else {
        panic!("RGB expected");
    };
    assert!(orange_r > orange_g && orange_g > orange_b);
    let Color::Rgb(_, green, blue) = hsl_to_rgb(BLUE_HUE, 0.8, 0.5) else {
        panic!("RGB expected");
    };
    assert!(blue > green);
}

#[test]
fn active_beat_band_overrides_cursor_background_and_keeps_waveform_foreground() {
    let foreground = Color::Rgb(12, 200, 80);
    let source = vec![Span::styled(
        "abcdefghijklmnop",
        base_style().fg(foreground),
    )];
    let mut rendered = Vec::new();

    append_cell(
        &mut rendered,
        &source,
        false,
        true,
        Some(crate::playback::position::PlaybackBeat {
            measure: 1,
            beat: 2,
            beats_per_measure: 4,
        }),
        1,
        16,
    );

    assert_eq!(rendered.len(), 16);
    for (column, span) in rendered.iter().enumerate() {
        assert_eq!(span.style.fg, Some(foreground));
        if (8..12).contains(&column) {
            assert_eq!(span.style.bg, Some(playhead::ACTIVE_BEAT_BG));
        } else {
            assert_eq!(
                span.style.bg,
                Some(cmrt_tui_core::theme::cursor_highlight_bg(foreground))
            );
        }
    }
}
