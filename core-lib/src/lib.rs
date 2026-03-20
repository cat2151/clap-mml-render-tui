pub use clap_mml_play_server_core::{CoreConfig, host, load_entry, midi, patch_list, render};
pub use clap_mml_play_server_core::patch_list::{collect_patches, to_relative};
pub use clap_mml_play_server_core::pipeline;
pub use clap_mml_play_server_core::pipeline::{
    ensure_cmrt_dir, ensure_daw_dir, ensure_phrase_dir, mml_render, mml_render_for_cache,
    mml_str_to_smf_bytes, mml_to_play, mml_to_smf_bytes, play_samples, write_wav,
};

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
            patch_path: Some("/patches/Pad 1.fxp".into()),
            patches_dir: Some("/patches".into()),
            random_patch: false,
        };

        assert_eq!(config.output_midi, "out.mid");
        assert_eq!(config.output_wav, "out.wav");
        assert_eq!(config.sample_rate, 44_100.0);
        assert_eq!(config.buffer_size, 512);
        assert_eq!(config.patch_path.as_deref(), Some("/patches/Pad 1.fxp"));
        assert_eq!(config.patches_dir.as_deref(), Some("/patches"));
        assert!(!config.random_patch);
    }

    #[test]
    fn reexports_patch_helpers() {
        assert_eq!(
            to_relative("/patches", Path::new("/patches/Pads/Pad 1.fxp")),
            "Pads/Pad 1.fxp"
        );
    }
}
