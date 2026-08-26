use std::path::Path;

use super::workspace_cache_dir;
use crate::WorkspaceKind;

#[test]
fn workspace_cache_directory_keeps_persistent_path_and_nests_daily() {
    let plugin_namespace = Path::new("daw_cache").join("Surge XT");

    assert_eq!(
        workspace_cache_dir(&plugin_namespace, WorkspaceKind::Persistent),
        plugin_namespace
    );
    assert_eq!(
        workspace_cache_dir(&plugin_namespace, WorkspaceKind::Daily),
        plugin_namespace.join("daily")
    );
}
