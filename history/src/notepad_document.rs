use anyhow::{ensure, Context, Result};

/// notepad の本文とカーソル位置。画面・演奏設定とは独立して保存する。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NotepadDocument {
    #[serde(default)]
    pub cursor: usize,
    pub lines: Vec<String>,
}

impl Default for NotepadDocument {
    fn default() -> Self {
        Self {
            cursor: 0,
            lines: super::helpers::default_lines(),
        }
    }
}

/// 未作成の場合だけ既定の本文を返す。既存ファイルの読み込み失敗は呼び出し元へ返す。
pub fn load_notepad_document() -> Result<NotepadDocument> {
    let path = super::paths::notepad_document_path().context("notepad の保存先を取得できません")?;
    let document: NotepadDocument = super::helpers::read_json_or_default(&path)?;
    ensure!(
        !document.lines.is_empty(),
        "notepad の本文が空配列です: {}",
        path.display()
    );
    Ok(document)
}

pub fn save_notepad_document(document: &NotepadDocument) -> Result<()> {
    let path = super::paths::notepad_document_path().context("notepad の保存先を取得できません")?;
    ensure!(
        !document.lines.is_empty(),
        "notepad の本文が空配列です: {}",
        path.display()
    );
    super::helpers::write_json(&path, document)
}

#[cfg(test)]
mod tests;
