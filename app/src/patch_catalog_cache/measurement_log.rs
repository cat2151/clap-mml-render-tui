//! 計測中のpatch load結果を1件ずつ追記するJSONL。
//!
//! `catalog.json`は全patchの計測が揃わないとTUIが読めないため、途中経過はここへ逃がす。
//! 計測が中断しても、次回の構築はこのfileから計測済みのpatchを拾って続きから計測する。

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cmrt_tui_core::patch_load::PatchLoadMeasurement;
use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "load-measurements.jsonl";

#[derive(Serialize, Deserialize)]
struct Entry {
    display: String,
    #[serde(flatten)]
    measurement: PatchLoadMeasurement,
}

pub(super) fn path_next_to(catalog_path: &Path) -> PathBuf {
    catalog_path.with_file_name(FILE_NAME)
}

/// 読めない行は捨てる。killされた直後の最終行は書きかけのことがある。
pub(super) fn read(path: &Path) -> BTreeMap<String, PatchLoadMeasurement> {
    let Ok(file) = File::open(path) else {
        return BTreeMap::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<Entry>(&line).ok())
        .map(|entry| (entry.display, entry.measurement))
        .collect()
}

pub(super) struct Writer {
    file: File,
}

impl Writer {
    pub(super) fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("計測logを開けません: {}", path.display()))?;
        Ok(Self { file })
    }

    pub(super) fn append(
        &mut self,
        display: &str,
        measurement: &PatchLoadMeasurement,
    ) -> Result<()> {
        let mut line = serde_json::to_string(&Entry {
            display: display.to_string(),
            measurement: measurement.clone(),
        })?;
        line.push('\n');
        self.file.write_all(line.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
