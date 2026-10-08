use std::{fs, path::Path};

use crate::{prepare_clap_patch_state, PatchStateError};

#[test]
fn reexport_prepares_owned_state_with_the_shared_error_type() {
    let prepare: fn(&str, &Path) -> Result<Vec<u8>, clap_mml_play_server_core::PatchStateError> =
        prepare_clap_patch_state;
    let path = std::env::temp_dir().join(format!(
        "cmrt-patches-shared-state-{}.fxp",
        std::process::id()
    ));
    let xml = b"<patch/>";
    let mut state = vec![0; 32];
    state[..4].copy_from_slice(b"sub3");
    state[4..8].copy_from_slice(&(xml.len() as u32).to_le_bytes());
    state.extend_from_slice(xml);
    let mut fxp = vec![0; 60];
    fxp[..4].copy_from_slice(b"CcnK");
    fxp[8..12].copy_from_slice(b"FPCh");
    fxp[16..20].copy_from_slice(b"cjs3");
    fxp[56..60].copy_from_slice(&(state.len() as u32).to_be_bytes());
    fxp.extend_from_slice(&state);
    fs::write(&path, fxp).unwrap();

    let result = prepare("org.surge-synth-team.surge-xt", &path);
    fs::remove_file(&path).unwrap();
    assert_eq!(result.unwrap(), state);

    let error: PatchStateError = prepare("unsupported", &path).unwrap_err();
    assert!(matches!(
        error,
        PatchStateError::UnsupportedPlugin { plugin_id } if plugin_id == "unsupported"
    ));
}
