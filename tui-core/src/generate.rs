//! `g` キーで挿入する既定フレーズ（notepad / DAW 共通）。

pub const DEFAULT_GENERATE_PHRASES: [&str; 2] = ["c1", "cfg1"];

pub fn pick_default_generate_phrase() -> &'static str {
    let index = crate::random::random_index(DEFAULT_GENERATE_PHRASES.len())
        .expect("default generate phrases should never be empty");
    DEFAULT_GENERATE_PHRASES[index]
}
