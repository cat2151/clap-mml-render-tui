use super::*;

fn stats(samples: &[f32]) -> RenderStats {
    RenderStats::of(samples, 48_000, "(test)")
}

#[test]
fn silence_is_reported_as_silence() {
    assert!(stats(&[0.0; 64]).is_silent());
    assert!(!stats(&[0.0, 0.5, 0.0, -0.5]).is_silent());
}

#[test]
fn the_length_is_counted_in_stereo_frames() {
    let one_second = stats(&vec![0.1_f32; 48_000 * 2]);
    assert_eq!(one_second.frames, 48_000);
    assert_eq!(one_second.duration_ms, 1000);
}

#[test]
fn the_same_samples_have_the_same_digest() {
    assert_eq!(digest(&[0.1, -0.2, 0.3]), digest(&[0.1, -0.2, 0.3]));
}

#[test]
fn one_changed_sample_changes_the_digest() {
    // 「音色を替えたのに出音が変わらない」を捕まえるための指紋なので、
    // 丸めて同じ値にしてはいけない。
    assert_ne!(digest(&[0.1, -0.2, 0.3]), digest(&[0.1, -0.2, 0.300_001]));
}

#[test]
fn an_identical_render_has_no_difference() {
    let left = stats(&[0.1, -0.2, 0.3, 0.4]);
    let right = stats(&[0.1, -0.2, 0.3, 0.4]);
    assert_eq!(diff_ratio(&left, &right), 0.0);
}

#[test]
fn a_completely_different_render_is_far_apart() {
    let left = stats(&[0.5, 0.5, 0.5, 0.5]);
    let right = stats(&[-0.5, -0.5, -0.5, -0.5]);
    assert!(diff_ratio(&left, &right) > 1.0);
}

#[test]
fn the_tail_of_the_longer_render_counts_as_a_difference() {
    // 短いほうへ切り詰めると「和音のほうが長く鳴っている」ぶんが差から消える。
    let short = stats(&[0.5, 0.5]);
    let long = stats(&[0.5, 0.5, 0.5, 0.5]);
    assert!(diff_ratio(&short, &long) > 0.5);
}

#[test]
fn two_silent_renders_do_not_divide_by_zero() {
    let left = stats(&[0.0; 8]);
    let right = stats(&[0.0; 8]);
    assert_eq!(diff_ratio(&left, &right), 0.0);
}

#[test]
fn distinct_digests_count_the_different_renders() {
    let a = stats(&[0.1, 0.2]).digest;
    let b = stats(&[0.1, 0.2]).digest;
    let c = stats(&[0.3, 0.4]).digest;
    assert_eq!(distinct_digests(&[a, b, c]), 2);
}

#[test]
fn a_patch_display_path_becomes_a_safe_file_name() {
    assert_eq!(file_stem_for("AR Accent Arp.vvp"), "AR_Accent_Arp_vvp");
    assert_eq!(
        file_stem_for("Dexed_01.syx/00 Say Again."),
        "Dexed_01_syx_00_Say_Again"
    );
}

#[test]
fn a_name_that_is_only_separators_still_produces_a_file_name() {
    assert_eq!(file_stem_for("///"), "render");
}

#[test]
fn a_multibyte_name_is_cut_by_characters() {
    // バイトで切ると panic する。
    let long = "あ".repeat(200);
    assert!(!file_stem_for(&long).is_empty());
}
