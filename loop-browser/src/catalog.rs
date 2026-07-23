use super::*;

impl LoopBrowser {
    pub fn rebuild_wav_categories(&mut self) {
        self.wav_categories = self
            .wav_analyses
            .iter()
            .filter_map(|(wav, _)| {
                self.metadata
                    .value
                    .category_for_wav(wav)
                    .map(|category| (wav.lookup_key(), category.to_string()))
            })
            .collect();
    }

    pub fn category_for_wav(&self, wav: &LoopWavId) -> Option<&str> {
        self.wav_categories
            .get(&wav.lookup_key())
            .map(String::as_str)
    }
}
