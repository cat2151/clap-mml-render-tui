struct TempFileGuard(std::path::PathBuf);

impl TempFileGuard {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(name);
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_file(&self.0) {
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "failed to remove temp config test file {}: {err}",
                    self.0.display()
                );
            }
        }
    }
}

#[test]
fn configured_editors_uses_app_default_when_unset() {
    let path = TempFileGuard::new(&format!(
        "cmrt-app-config-editors-{}-unset.toml",
        std::process::id()
    ));
    std::fs::write(path.path(), "sample_rate = 48000\n").unwrap();

    let editors = crate::config_editor::configured_editors(path.path()).unwrap();

    assert_eq!(
        editors,
        vec![
            "fresh".to_string(),
            "zed".to_string(),
            "code".to_string(),
            "edit".to_string(),
            "nano".to_string(),
            "vim".to_string()
        ]
    );
}

#[test]
fn configured_editors_uses_toml_value_when_set() {
    let path = TempFileGuard::new(&format!(
        "cmrt-app-config-editors-{}-set.toml",
        std::process::id()
    ));
    std::fs::write(path.path(), r#"editors = ["code", "vim"]"#).unwrap();

    let editors = crate::config_editor::configured_editors(path.path()).unwrap();

    assert_eq!(editors, vec!["code".to_string(), "vim".to_string()]);
}
