pub use clap_mml_play_server_core::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn reexports_core_config() {
        let config = CoreConfig {
            output_midi: "out.mid".into(),
            output_wav: "out.wav".into(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patch_path: None,
            patches_dir: Some("/patches".into()),
            random_patch: false,
        };

        assert_eq!(config.buffer_size, 512);
        assert_eq!(config.patches_dir.as_deref(), Some("/patches"));
    }

    #[test]
    fn reexports_patch_helpers() {
        assert_eq!(
            to_relative("/patches", Path::new("/patches/Pads/Pad 1.fxp")),
            "Pads/Pad 1.fxp"
        );
    }
}
