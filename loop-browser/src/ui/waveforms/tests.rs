use super::*;

#[test]
fn one_shot_uses_only_its_real_duration_on_the_span_timeline() {
    assert_eq!(
        waveform_output_columns(LoopWavKind::OneShot, 4.785, None, 120.0, 4, 2.0, 64),
        39
    );
}

#[test]
fn one_shot_ending_on_its_boundary_uses_the_whole_span() {
    assert_eq!(
        waveform_output_columns(LoopWavKind::OneShot, 4.0, None, 120.0, 2, 2.0, 32),
        32
    );
}

#[test]
fn loop_uses_the_whole_span_when_stretched_audio_fills_it() {
    assert_eq!(
        waveform_output_columns(LoopWavKind::Loop, 4.0, Some(120.0), 120.0, 2, 2.0, 32),
        32
    );
}

#[test]
fn logged_short_guitar_leaves_the_same_trailing_silence_as_playback() {
    assert_eq!(
        waveform_output_columns(LoopWavKind::Loop, 0.862_063, Some(139.2), 120.0, 1, 2.0, 16),
        8
    );
}

#[test]
fn loop_duration_is_adjusted_by_the_playback_time_ratio() {
    assert_eq!(
        waveform_output_columns(LoopWavKind::Loop, 1.6, Some(150.0), 120.0, 1, 2.0, 16),
        16
    );
}

#[test]
fn longer_loop_timeline_is_not_compressed_before_playback_crops_it() {
    assert_eq!(
        waveform_output_columns(LoopWavKind::Loop, 4.0, Some(120.0), 120.0, 1, 2.0, 16),
        32
    );
}

#[test]
fn zero_width_has_no_waveform_columns() {
    assert_eq!(
        waveform_output_columns(LoopWavKind::Loop, 2.0, Some(120.0), 120.0, 1, 2.0, 0),
        0
    );
}

#[test]
fn columns_after_a_one_shot_are_silent_instead_of_stretched_waveform() {
    let waveform = LoopWaveform {
        rms_db_tenths: vec![-60; 32],
        spectral_flux: vec![0; 32],
        centroid_motion_millioctaves: 0,
    };
    let waveform_len =
        waveform_output_columns(LoopWavKind::OneShot, 4.785, None, 120.0, 4, 2.0, 64);

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
fn columns_after_a_short_loop_are_silent_instead_of_stretched_waveform() {
    let waveform = LoopWaveform {
        rms_db_tenths: vec![-60; 32],
        spectral_flux: vec![0; 32],
        centroid_motion_millioctaves: 0,
    };
    let waveform_len =
        waveform_output_columns(LoopWavKind::Loop, 1.0, Some(120.0), 120.0, 1, 2.0, 16);
    let scale = display_scale(WaveformDisplayScale::default(), &waveform, waveform_len, 16);

    assert_ne!(
        render_timeline_column(&waveform, scale, waveform_len - 1, waveform_len).content,
        "·"
    );
    assert_eq!(
        render_timeline_column(&waveform, scale, waveform_len, waveform_len).content,
        "·"
    );
}

#[test]
fn cropped_loop_shows_only_the_source_prefix_that_playback_uses() {
    let waveform = LoopWaveform {
        rms_db_tenths: vec![-400, -300, -200, -100],
        spectral_flux: vec![0; 4],
        centroid_motion_millioctaves: 0,
    };
    let waveform_len =
        waveform_output_columns(LoopWavKind::Loop, 4.0, Some(120.0), 120.0, 1, 2.0, 2);
    let scale = display_scale(WaveformDisplayScale::default(), &waveform, waveform_len, 2);

    assert_eq!(waveform_len, 4);
    assert_eq!(
        render_timeline_column(&waveform, scale, 1, waveform_len).content,
        render_column(&waveform, scale, 1, waveform.len()).content
    );
}

#[test]
fn internal_silence_stays_on_its_original_timeline() {
    let waveform = LoopWaveform {
        rms_db_tenths: vec![-120, SILENCE_DB_TENTHS, -60],
        spectral_flux: vec![0; 3],
        centroid_motion_millioctaves: 0,
    };
    let scale = display_scale(WaveformDisplayScale::default(), &waveform, 3, 3);

    assert_ne!(render_column(&waveform, scale, 0, 3).content, "·");
    assert_eq!(render_column(&waveform, scale, 1, 3).content, "·");
    assert_ne!(render_column(&waveform, scale, 2, 3).content, "·");
}

#[test]
fn each_loop_expands_its_own_rms_range_to_all_glyph_heights() {
    let waveform = LoopWaveform {
        rms_db_tenths: vec![-300, -200, -100],
        spectral_flux: vec![0; 3],
        centroid_motion_millioctaves: 0,
    };
    let scale = display_scale(WaveformDisplayScale::default(), &waveform, 3, 3);

    assert_eq!(render_column(&waveform, scale, 0, 3).content, "▁");
    assert_eq!(render_column(&waveform, scale, 2, 3).content, "█");
}

#[test]
fn a_flat_loop_uses_the_lowest_audible_glyph() {
    let waveform = LoopWaveform {
        rms_db_tenths: vec![-120; 4],
        spectral_flux: vec![0; 4],
        centroid_motion_millioctaves: 0,
    };
    let scale = display_scale(WaveformDisplayScale::default(), &waveform, 4, 4);

    assert_eq!(render_column(&waveform, scale, 0, 4).content, "▁");
}

#[test]
fn logged_guitar_has_minimum_and_maximum_glyphs_after_column_aggregation() {
    let waveform = LoopWaveform {
        rms_db_tenths: vec![
            -143, -296, -140, -231, -146, -223, -134, -226, -119, -227, -126, -284, -121, -267,
            -140, -241, -129, -273, -130, -247, -113, -219, -130, -219, -131, -286, -131, -234,
            -129, -163, -142, -171,
        ],
        spectral_flux: vec![0; 32],
        centroid_motion_millioctaves: 0,
    };
    let scale = display_scale(WaveformDisplayScale::default(), &waveform, 16, 16);
    let glyphs = (0..16)
        .map(|column| {
            render_timeline_column(&waveform, scale, column, 16)
                .content
                .into_owned()
        })
        .collect::<Vec<_>>()
        .concat();

    assert!(glyphs.contains('▁'), "{glyphs}");
    assert!(glyphs.contains('█'), "{glyphs}");
}

/// セル幅を解析の bin 数まで広げたときは 1 文字が 32 分音符 1 個に 1 対 1 で対応する。
/// 隣り合う bin を混ぜていないことを、交互の大小が潰れないことで確かめる。
#[test]
fn the_widest_cell_maps_one_column_to_one_thirty_second_note() {
    let bins = cmrt_loop_domain::loop_waveform::WAVEFORM_BINS_PER_MEASURE;
    let waveform = LoopWaveform {
        rms_db_tenths: (0..bins)
            .map(|bin| if bin % 2 == 0 { -100 } else { -400 })
            .collect(),
        spectral_flux: vec![0; bins],
        centroid_motion_millioctaves: 0,
    };
    let glyphs = |output_len: usize| {
        let scale = display_scale(
            WaveformDisplayScale::default(),
            &waveform,
            output_len,
            output_len,
        );
        (0..output_len)
            .map(|column| {
                render_timeline_column(&waveform, scale, column, output_len)
                    .content
                    .into_owned()
            })
            .collect::<Vec<_>>()
            .concat()
    };

    assert_eq!(glyphs(bins), "█▁".repeat(bins / 2));
    // 半分の幅では 2 bin ずつ混ざるので、同じ波形が平坦に見えてしまう。
    assert_eq!(glyphs(bins / 2), "▁".repeat(bins / 2));
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
