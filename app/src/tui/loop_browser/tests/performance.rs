use super::*;

#[test]
fn analysis_lookup_is_indexed_for_a_realistic_library() {
    const WAV_COUNT: usize = 6_914;
    let browser = browser_with_direct_wavs(WAV_COUNT);
    let wav = browser.wav_analyses.last().unwrap().0.clone();

    assert_eq!(browser.wav_analysis_indices.len(), WAV_COUNT);
    let started_at = std::time::Instant::now();
    for _ in 0..1_000 {
        assert!(browser.analysis_for_wav(&wav).is_some());
    }
    assert!(
        started_at.elapsed() < std::time::Duration::from_millis(250),
        "indexed lookup exceeded budget: {:?}",
        started_at.elapsed()
    );
}
