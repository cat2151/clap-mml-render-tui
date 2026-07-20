use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

const METADATA_VERSION: u32 = 1;
const METADATA_FILE_NAME: &str = "loop_browser.toml";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LoopDirId {
    pub(crate) root: String,
    pub(crate) relative: String,
}

impl LoopDirId {
    pub(crate) fn new(root: &Path, relative: &Path) -> Self {
        Self {
            root: root.to_string_lossy().into_owned(),
            relative: relative.to_string_lossy().into_owned(),
        }
    }

    fn lookup_key(&self) -> (String, String) {
        (
            normalize_path_text(&self.root),
            normalize_path_text(&self.relative),
        )
    }

    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.lookup_key() == other.lookup_key()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LoopCategoryAssignment {
    pub(crate) root: String,
    pub(crate) relative: String,
    pub(crate) category: String,
}

impl LoopCategoryAssignment {
    fn dir_id(&self) -> LoopDirId {
        LoopDirId {
            root: self.root.clone(),
            relative: self.relative.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LoopBrowserMetadata {
    version: u32,
    #[serde(default)]
    pub(crate) favorite_dirs: Vec<LoopDirId>,
    #[serde(default)]
    pub(crate) category_assignments: Vec<LoopCategoryAssignment>,
}

impl Default for LoopBrowserMetadata {
    fn default() -> Self {
        Self {
            version: METADATA_VERSION,
            favorite_dirs: Vec::new(),
            category_assignments: Vec::new(),
        }
    }
}

impl LoopBrowserMetadata {
    pub(crate) fn load_from(path: &Path) -> Result<Self> {
        load_from_path(path)
    }

    pub(crate) fn save_to(&self, path: &Path) -> Result<()> {
        save_to_path(path, self)
    }

    pub(crate) fn is_favorite(&self, dir: &LoopDirId) -> bool {
        self.favorite_dirs.iter().any(|item| item.matches(dir))
    }

    pub(crate) fn toggle_favorite(&mut self, dir: &LoopDirId) -> bool {
        if let Some(index) = self.favorite_dirs.iter().position(|item| item.matches(dir)) {
            self.favorite_dirs.remove(index);
            false
        } else {
            self.favorite_dirs.push(dir.clone());
            true
        }
    }

    pub(crate) fn category_for<'a>(&'a self, dir: &LoopDirId) -> Option<&'a str> {
        self.category_assignments
            .iter()
            .find(|assignment| assignment.dir_id().matches(dir))
            .map(|assignment| assignment.category.as_str())
    }

    pub(crate) fn toggle_category(&mut self, dir: &LoopDirId, category: &str) -> Option<String> {
        if let Some(index) = self
            .category_assignments
            .iter()
            .position(|assignment| assignment.dir_id().matches(dir))
        {
            if self.category_assignments[index].category == category {
                self.category_assignments.remove(index);
                return None;
            }
            self.category_assignments[index].category = category.to_string();
            return Some(category.to_string());
        }
        self.category_assignments.push(LoopCategoryAssignment {
            root: dir.root.clone(),
            relative: dir.relative.clone(),
            category: category.to_string(),
        });
        Some(category.to_string())
    }
}

pub(crate) fn category_keys(categories: &[String]) -> Vec<(char, String)> {
    let mut used = [false; 26];
    categories
        .iter()
        .map(|category| {
            let from_name = category.chars().find_map(|character| {
                let character = character.to_ascii_lowercase();
                if !character.is_ascii_lowercase() {
                    return None;
                }
                let index = (character as u8 - b'a') as usize;
                (!used[index]).then_some((index, character))
            });
            let (index, key) = from_name.unwrap_or_else(|| {
                let index = used.iter().position(|is_used| !is_used).unwrap_or(0);
                (index, (b'a' + index as u8) as char)
            });
            used[index] = true;
            (key, category.clone())
        })
        .collect()
}

pub(crate) fn metadata_path() -> Result<PathBuf> {
    crate::config::config_app_dir()
        .map(|dir| dir.join(METADATA_FILE_NAME))
        .ok_or_else(|| anyhow::anyhow!("システムの設定ディレクトリが取得できません"))
}

fn load_from_path(path: &Path) -> Result<LoopBrowserMetadata> {
    if !path.exists() {
        return Ok(LoopBrowserMetadata::default());
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("loop browser metadataを読めません: {}", path.display()))?;
    let metadata: LoopBrowserMetadata = toml::from_str(&text)
        .with_context(|| format!("loop browser metadataが壊れています: {}", path.display()))?;
    if metadata.version != METADATA_VERSION {
        anyhow::bail!(
            "loop browser metadataのversionが一致しません（file: {}, expected: {}）",
            metadata.version,
            METADATA_VERSION
        );
    }
    validate_metadata(&metadata)?;
    Ok(metadata)
}

fn validate_metadata(metadata: &LoopBrowserMetadata) -> Result<()> {
    for dir in &metadata.favorite_dirs {
        validate_dir_id(dir)?;
    }
    for assignment in &metadata.category_assignments {
        validate_dir_id(&assignment.dir_id())?;
        if assignment.category.trim().is_empty() {
            anyhow::bail!("loop browser metadataに空のカテゴリ割当があります");
        }
    }
    Ok(())
}

fn validate_dir_id(dir: &LoopDirId) -> Result<()> {
    if dir.root.trim().is_empty() {
        anyhow::bail!("loop browser metadataに空のrootがあります");
    }
    let relative = Path::new(&dir.relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        anyhow::bail!(
            "loop browser metadataに不正な相対dirがあります: {}",
            dir.relative
        );
    }
    Ok(())
}

fn save_to_path(path: &Path, metadata: &LoopBrowserMetadata) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "loop browser metadata directoryを作れません: {}",
                parent.display()
            )
        })?;
    }
    let text = toml::to_string_pretty(metadata)?;
    let temp_path = unique_temp_path(path);
    let write_result = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .and_then(|mut file| {
            use std::io::Write as _;
            file.write_all(text.as_bytes())?;
            file.sync_all()
        });
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error).with_context(|| {
            format!(
                "一時loop browser metadataを書けません: {}",
                temp_path.display()
            )
        });
    }
    if let Err(error) = replace_file(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("loop browser metadataを置換できません: {}", path.display()));
    }
    Ok(())
}

fn normalize_path_text(text: &str) -> String {
    let normalized = Path::new(text).components().collect::<PathBuf>();
    let normalized = normalized.to_string_lossy().into_owned();
    if cfg!(windows) {
        normalized.to_lowercase()
    } else {
        normalized
    }
}

fn unique_temp_path(path: &Path) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.with_extension(format!("tmp-{}-{nonce}", std::process::id()))
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let from = from
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let to = to
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_keys_use_name_characters_then_unused_alphabet() {
        let categories = ["Guitar", "glock", "lead", "123", "gale"].map(str::to_string);
        let keys = category_keys(&categories)
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>();
        assert_eq!(keys, ['g', 'l', 'e', 'a', 'b']);
    }

    #[test]
    fn metadata_round_trip_and_rejects_bad_data() {
        let dir = std::env::temp_dir().join(format!(
            "cmrt-loop-browser-metadata-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("loop_browser.toml");
        let mut metadata = LoopBrowserMetadata::default();
        let id = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass"));
        assert!(metadata.toggle_favorite(&id));
        metadata.toggle_category(&id, "bass");
        save_to_path(&path, &metadata).unwrap();
        assert_eq!(load_from_path(&path).unwrap(), metadata);

        std::fs::write(&path, "version = 99").unwrap();
        assert!(load_from_path(&path).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
