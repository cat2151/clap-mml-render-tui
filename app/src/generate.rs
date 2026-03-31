use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const DEFAULT_GENERATE_PHRASES: [&str; 2] = ["c1", "cfg1"];

pub(crate) fn pick_default_generate_phrase() -> &'static str {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let index = (ns % DEFAULT_GENERATE_PHRASES.len() as u128) as usize;
    DEFAULT_GENERATE_PHRASES[index]
}
