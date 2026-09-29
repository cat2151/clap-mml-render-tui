mod handler;
mod heavy_preview;
mod normal;
mod open;

use crate::{
    filter_patches_by_display_path, PatchLoadState, PATCH_FILTER_QUERY_JSON_KEY, PATCH_JSON_KEY,
};
use cmrt_core::EFFECT_CHAIN_JSON_KEY;
use cmrt_patch_select::auto_reverb::{HostChain, AUTO_REVERB_JSON_KEY, MANUAL_REVERB_JSON_KEY};
use cmrt_patch_select::PatchSelect;
use mmlabc_to_smf::mml_preprocessor;
use serde_json::{Map, Value};

use crate::NotepadScreen;
pub(crate) use heavy_preview::HeavyPreview;

const PATCH_SELECT_PREVIEW_FALLBACK_PHRASE: &str = "c";

/// 行頭 JSON のうち effect に関わる key。書き出す順でもある。
const EFFECT_JSON_KEYS: [&str; 3] = [
    EFFECT_CHAIN_JSON_KEY,
    AUTO_REVERB_JSON_KEY,
    MANUAL_REVERB_JSON_KEY,
];

/// 現在行のフレーズ・filter 語・effect chain。音色名を差し込むと試聴 MML になる。
pub(crate) struct PatchSelectPreviewMml {
    phrase: String,
    filter_query: Option<String>,
    chain: HostChain,
}

impl PatchSelectPreviewMml {
    /// filter 語と chain は行のものを入れる。`Enter` 後の行と JSON 文字列が一致し、
    /// 確定直後の再生が試聴で作ったキャッシュに当たる。
    ///
    /// `auto_reverb` は行の chain の auto reverb の段として入れる。
    pub(crate) fn for_patch(&self, patch_name: &str, auto_reverb: Option<Value>) -> String {
        let json = NotepadScreen::build_patch_json_with_filter_query(
            patch_name,
            self.filter_query.as_deref(),
            &effect_keys(&self.chain, auto_reverb.as_ref()),
        );
        format!("{json} {}", self.phrase)
    }

    /// `select` が決める auto reverb を載せた試聴 MML。
    pub(crate) fn for_selector_patch(&self, select: &PatchSelect<'_>, patch_name: &str) -> String {
        self.for_patch(patch_name, select.auto_reverb_stage(patch_name))
    }
}

/// `chain` に `auto_reverb` を入れたときの、行頭 JSON の effect の key。
pub(crate) fn effect_keys(chain: &HostChain, auto_reverb: Option<&Value>) -> Map<String, Value> {
    let mut keys = Map::new();
    chain.write_into(&mut keys, auto_reverb);
    keys
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
        Self::build_patch_json_with_filter_query(patch_name, None, &Map::new())
    }

    /// `effects` は [`EFFECT_JSON_KEYS`] の key だけを見る。
    fn build_patch_json_with_filter_query(
        patch_name: &str,
        filter_query: Option<&str>,
        effects: &Map<String, Value>,
    ) -> String {
        let patch_name =
            serde_json::to_string(patch_name).unwrap_or_else(|_| format!("\"{}\"", patch_name));
        let mut json = format!(r#"{{"{PATCH_JSON_KEY}": {patch_name}"#);
        if let Some(filter_query) = filter_query
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            let filter_query = serde_json::to_string(filter_query)
                .unwrap_or_else(|_| format!("\"{}\"", filter_query));
            json.push_str(&format!(
                r#", "{PATCH_FILTER_QUERY_JSON_KEY}": {filter_query}"#
            ));
        }
        for key in EFFECT_JSON_KEYS {
            if let Some(value) = effects.get(key) {
                json.push_str(&format!(r#", "{key}": {value}"#));
            }
        }
        json.push('}');
        json
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

    fn current_line_patch_json(&self) -> Option<Value> {
        let line = self.editor.lines.get(self.editor.cursor)?;
        Self::extract_patch_json_value(line)
    }

    /// 現在行の行頭 JSON の effect chain と、auto reverb の扱い。
    pub(crate) fn current_line_host_chain(&self) -> HostChain {
        HostChain::from_json(
            self.current_line_patch_json().as_ref(),
            self.effect_plugins.catalog(),
        )
    }

    /// 現在行の行頭 JSON にある effect の key をそのまま写したもの。音色だけを差し替えるときに使う。
    pub(super) fn current_line_effect_keys(&self) -> Map<String, Value> {
        let json = self.current_line_patch_json();
        EFFECT_JSON_KEYS
            .into_iter()
            .filter_map(|key| Some((key.to_string(), json.as_ref()?.get(key)?.clone())))
            .collect()
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

    /// 行頭 JSON の音色と effect の key を差し替える。filter 語は行のものを残す。
    pub(super) fn replace_current_line_patch(
        &mut self,
        patch_name: &str,
        effects: &Map<String, Value>,
    ) {
        let filter_query = self.current_line_patch_filter_query();
        self.replace_current_line_patch_with_filter(patch_name, filter_query.as_deref(), effects);
    }

    pub(super) fn replace_current_line_patch_with_filter(
        &mut self,
        patch_name: &str,
        filter_query: Option<&str>,
        effects: &Map<String, Value>,
    ) {
        let json = Self::build_patch_json_with_filter_query(patch_name, filter_query, effects);
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
            chain: self.current_line_host_chain(),
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
                // 重い patch の先読みは render worker を長く塞ぎ、今の試聴を待たせる。選んだときだけ鳴らす。
                select
                    .filtered_display(index)
                    .filter(|patch_name| {
                        !select
                            .load_measurement(patch_name)
                            .is_some_and(|measurement| measurement.is_heavy_offline_load())
                    })
                    .map(|patch_name| preview_mml.for_selector_patch(select, patch_name))
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
        if self.selected_heavy_sample_bytes().is_some() {
            // cache に無い重い音色は Space で確認してから鳴らす（heavy_preview）。
            self.stop_preview_for_heavy_patch();
            self.prefetch_patch_select_navigation_audio_cache(preferred_delta);
            return;
        }
        let Some(patch_name) = self.patch_select_selected_patch_name() else {
            return;
        };
        let Some(select) = self.patch_select.as_ref() else {
            return;
        };
        let Some(mml) = self
            .patch_select_preview_mml_builder()
            .map(|preview_mml| preview_mml.for_selector_patch(select, &patch_name))
        else {
            return;
        };
        Self::log_notepad_event(format!("tone-select preview patch={patch_name:?}"));
        self.record_notepad_history(&mml);
        self.play_mml(mml);
        self.prefetch_patch_select_navigation_audio_cache(preferred_delta);
    }
}

#[cfg(test)]
mod tests;
