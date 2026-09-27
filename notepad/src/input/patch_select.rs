mod handler;
mod normal;
mod open;

use crate::{
    filter_patches_by_display_path, PatchLoadState, PATCH_FILTER_QUERY_JSON_KEY, PATCH_JSON_KEY,
};
use mmlabc_to_smf::mml_preprocessor;
use serde_json::Value;

use crate::NotepadScreen;

const PATCH_SELECT_PREVIEW_FALLBACK_PHRASE: &str = "c";

/// 現在行のフレーズと filter 語。音色名を差し込むと試聴 MML になる。
pub(crate) struct PatchSelectPreviewMml {
    phrase: String,
    filter_query: Option<String>,
}

impl PatchSelectPreviewMml {
    /// filter 語は行のものを入れる。`Enter` 後の行と JSON 文字列が一致し、
    /// 確定直後の再生が試聴で作ったキャッシュに当たる。
    pub(crate) fn for_patch(&self, patch_name: &str) -> String {
        let json = NotepadScreen::build_patch_json_with_filter_query(
            patch_name,
            self.filter_query.as_deref(),
        );
        format!("{json} {}", self.phrase)
    }
}

impl<'a> NotepadScreen<'a> {
    fn resolve_loaded_patch_name(&self, patch_name: &str) -> Option<String> {
        let state = self.patch_load_state.lock().unwrap();
        match &*state {
            PatchLoadState::Ready(snapshot) => {
                cmrt_patches::resolve_display_patch_name(snapshot.pairs(), patch_name)
            }
            PatchLoadState::Loading | PatchLoadState::Err(_) => None,
        }
    }

    pub(crate) fn normalize_patch_phrase_store_key(&mut self, patch_name: String) -> String {
        let Some(resolved) = self.resolve_loaded_patch_name(&patch_name) else {
            return patch_name;
        };
        if resolved != patch_name
            && cmrt_history::rename_patch_phrase_store_key(
                &mut self.patch_phrase_store,
                &patch_name,
                &resolved,
            )
        {
            self.patch_phrase_store_dirty = true;
        }
        resolved
    }

    pub(crate) fn normalize_patch_phrase_store_for_available_patches(
        &mut self,
        pairs: &[(String, String)],
    ) {
        if cmrt_history::normalize_patch_phrase_store_for_available_patches(
            &mut self.patch_phrase_store,
            pairs,
        ) {
            self.patch_phrase_store_dirty = true;
        }
    }

    fn build_patch_json(patch_name: &str) -> String {
        Self::build_patch_json_with_filter_query(patch_name, None)
    }

    fn build_patch_json_with_filter_query(patch_name: &str, filter_query: Option<&str>) -> String {
        let patch_name =
            serde_json::to_string(patch_name).unwrap_or_else(|_| format!("\"{}\"", patch_name));
        match filter_query
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            Some(filter_query) => {
                let filter_query = serde_json::to_string(filter_query)
                    .unwrap_or_else(|_| format!("\"{}\"", filter_query));
                format!(
                    r#"{{"{PATCH_JSON_KEY}": {patch_name}, "{PATCH_FILTER_QUERY_JSON_KEY}": {filter_query}}}"#
                )
            }
            None => format!(r#"{{"{PATCH_JSON_KEY}": {patch_name}}}"#),
        }
    }

    fn extract_patch_json_value(mml: &str) -> Option<Value> {
        let preprocessed = mml_preprocessor::extract_embedded_json(mml);
        preprocessed
            .embedded_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<Value>(json).ok())
    }

    /// 保存済みの汎用 filter が無い場合に、現在 patch の親 dir 名を初期検索語にする。
    ///
    /// 戻り値は category 専用条件ではない。category、vendor、filename のすべてを対象にする
    /// 表示パス全文 filter へ、そのまま渡す query である。
    pub(crate) fn default_display_path_filter_query_from_parent_dir(
        patch_name: &str,
    ) -> Option<String> {
        let (parent, _) = patch_name.rsplit_once('/')?;
        let category = parent.rsplit('/').next()?.trim();
        if category.is_empty() {
            None
        } else {
            Some(category.to_lowercase())
        }
    }

    fn current_line_patch_filter_query(&self) -> Option<String> {
        self.editor.lines.get(self.editor.cursor).and_then(|line| {
            Self::extract_patch_json_value(line).and_then(|value| {
                value
                    .get(PATCH_FILTER_QUERY_JSON_KEY)
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
        })
    }

    fn has_matching_patches_for_query(&self, query: &str) -> bool {
        let state = self.patch_load_state.lock().unwrap();
        match &*state {
            PatchLoadState::Ready(snapshot) => {
                !filter_patches_by_display_path(snapshot.pairs(), query).is_empty()
            }
            PatchLoadState::Loading | PatchLoadState::Err(_) => false,
        }
    }

    pub(super) fn current_line_random_patch_filter_query(&self) -> Option<String> {
        self.current_line_patch_filter_query()
            .and_then(|query| self.has_matching_patches_for_query(&query).then_some(query))
            .or_else(|| {
                self.current_line_patch_name()
                    .and_then(|patch_name| {
                        Self::default_display_path_filter_query_from_parent_dir(&patch_name)
                    })
                    .filter(|query| self.has_matching_patches_for_query(query))
            })
    }

    pub(super) fn replace_current_line_patch(&mut self, patch_name: &str) {
        let filter_query = self.current_line_patch_filter_query();
        self.replace_current_line_patch_with_filter(patch_name, filter_query.as_deref());
    }

    pub(super) fn replace_current_line_patch_with_filter(
        &mut self,
        patch_name: &str,
        filter_query: Option<&str>,
    ) {
        let json = Self::build_patch_json_with_filter_query(patch_name, filter_query);
        let current = self.editor.lines[self.editor.cursor].clone();
        let replaced_parts = current
            .split(';')
            .map(|part| {
                let part = part.trim_start();
                let preprocessed = mml_preprocessor::extract_embedded_json(part);
                let remaining = preprocessed.remaining_mml.trim();
                if remaining.is_empty() {
                    String::new()
                } else {
                    format!("{json} {remaining}")
                }
            })
            .collect::<Vec<_>>();
        let replaced = replaced_parts.join(";");
        let has_content = replaced_parts.iter().any(|part| !part.trim().is_empty());
        self.editor.lines[self.editor.cursor] = if has_content {
            replaced
        } else {
            format!("{json} c")
        };
    }

    /// 試聴で鳴らすフレーズ。現在行の MML 部分で、空なら既定の 1 音。
    pub(super) fn patch_select_current_phrase(&self) -> Option<String> {
        let line = self.editor.lines.get(self.editor.cursor)?;
        let preprocessed = mml_preprocessor::extract_embedded_json(line);
        Some(match preprocessed.remaining_mml.trim() {
            "" => PATCH_SELECT_PREVIEW_FALLBACK_PHRASE.to_string(),
            remaining => remaining.to_string(),
        })
    }

    /// 音色選択の試聴 MML の材料。行の解析を 1 回で済ませ、音色ごとの MML はここから作る。
    pub(crate) fn patch_select_preview_mml_builder(&self) -> Option<PatchSelectPreviewMml> {
        Some(PatchSelectPreviewMml {
            phrase: self.patch_select_current_phrase()?,
            filter_query: self.current_line_patch_filter_query(),
        })
    }

    pub(crate) fn patch_select_selected_patch_name(&self) -> Option<String> {
        self.patch_select
            .as_ref()
            .and_then(|select| select.selected())
            .map(str::to_string)
    }

    fn prefetch_patch_select_navigation_audio_cache(&self, preferred_delta: Option<isize>) {
        let Some(select) = self.patch_select.as_ref() else {
            return;
        };
        let Some(preview_mml) = self.patch_select_preview_mml_builder() else {
            return;
        };
        self.prefetch_navigation_audio_cache(
            select.cursor(),
            select.filtered_len(),
            cmrt_patch_select::PAGE_STEP.unsigned_abs(),
            preferred_delta,
            |index| {
                select
                    .filtered_display(index)
                    .map(|patch_name| preview_mml.for_patch(patch_name))
            },
        );
    }

    pub(super) fn preview_selected_patch(&mut self) {
        self.preview_selected_patch_with_navigation_hint(None);
    }

    pub(crate) fn preview_selected_patch_with_navigation_hint(
        &mut self,
        preferred_delta: Option<isize>,
    ) {
        let Some(patch_name) = self.patch_select_selected_patch_name() else {
            return;
        };
        let Some(mml) = self
            .patch_select_preview_mml_builder()
            .map(|preview_mml| preview_mml.for_patch(&patch_name))
        else {
            return;
        };
        Self::log_notepad_event(format!("tone-select preview patch={patch_name:?}"));
        self.record_notepad_history(&mml);
        self.play_mml(mml);
        self.prefetch_patch_select_navigation_audio_cache(preferred_delta);
    }
}
