use crate::tui::{filter_patches, Mode, PatchLoadState, PlayState, TuiApp};

impl<'a> TuiApp<'a> {
    fn normalize_current_line_patch_json_if_known(&mut self) {
        let Some(current_patch_name) = self.current_line_patch_name() else {
            return;
        };
        let Some(raw_patch_name) = self
            .lines
            .get(self.cursor)
            .and_then(|line| Self::extract_patch_phrase(line))
            .map(|(patch_name, _)| patch_name)
        else {
            return;
        };
        if current_patch_name != raw_patch_name {
            self.replace_current_line_patch(&current_patch_name);
        }
    }

    pub(in crate::tui) fn play_mml(&mut self, mml: String) {
        #[cfg(test)]
        if self.entry_ptr == 0 {
            // new_for_test() では PluginEntry を持たないため、
            // テスト中は再生スレッドを起動せず play_state 更新だけを検証する。
            *self.play_state.lock().unwrap() = PlayState::Running(mml);
            return;
        }

        self.kick_play(mml);
    }

    fn prefetch_normal_navigation_audio_cache(&self, preferred_delta: Option<isize>) {
        self.prefetch_navigation_audio_cache(
            self.cursor,
            self.lines.len(),
            self.normal_page_size,
            preferred_delta,
            |index| {
                self.lines
                    .get(index)
                    .map(|line| line.trim().to_string())
                    .filter(|mml| !mml.is_empty())
            },
        );
    }

    pub(in crate::tui) fn prime_normal_mode_startup_cache(&self) {
        let Some(current_mml) = self
            .lines
            .get(self.cursor)
            .map(|line| line.trim().to_string())
            .filter(|mml| !mml.is_empty())
        else {
            return;
        };
        self.load_disk_cached_audio_into_memory_if_present(&current_mml);
        let navigation_targets = crate::ui_utils::predicted_navigation_indices(
            self.cursor,
            self.lines.len(),
            self.normal_page_size,
        )
        .into_iter()
        .filter_map(|index| {
            self.lines
                .get(index)
                .map(|line| line.trim().to_string())
                .filter(|mml| !mml.is_empty())
        })
        .collect::<Vec<_>>();
        self.prefetch_audio_cache_with_idle_fill(vec![current_mml], navigation_targets);
    }

    /// 現在バッファの全行について、有効なディスクキャッシュ WAV があれば `audio_cache` へロードする。
    /// プロセス起動直後に一度だけ呼ぶことを想定しており、パッチ選択確定時などに何度も呼ばれる
    /// `prime_normal_mode_startup_cache` からは呼ばない（呼ぶとディスク読み込みが繰り返されてしまう）。
    pub(in crate::tui) fn hydrate_all_lines_from_disk_cache_at_startup(&self) {
        for line in &self.lines {
            let mml = line.trim();
            if mml.is_empty() {
                continue;
            }
            self.load_disk_cached_audio_into_memory_if_present(mml);
        }
    }

    /// ディスクキャッシュに `mml` の妥当な WAV があれば `audio_cache` へ読み込む。
    /// 起動直後のオフラインレンダリング待ちを、前回セッションで書き出したキャッシュで
    /// スキップできるかを確認するための処理。
    fn load_disk_cached_audio_into_memory_if_present(&self, mml: &str) {
        if self.audio_cache.lock().unwrap().contains_key(mml) {
            return;
        }
        let Some(samples) =
            crate::tui::disk_cache::load_valid_cached_wav(mml, self.cfg.sample_rate as u32)
        else {
            return;
        };
        let mut cache = self.audio_cache.lock().unwrap();
        let mut cache_order = self.audio_cache_order.lock().unwrap();
        crate::tui::cache::try_insert_cache(
            &mut cache,
            &mut cache_order,
            mml.to_string(),
            samples,
            false,
        );
    }

    pub(in crate::tui::input) fn play_current_line(&mut self) {
        self.play_current_line_with_navigation_hint(None);
    }

    pub(in crate::tui::input) fn play_current_line_with_navigation_hint(
        &mut self,
        preferred_delta: Option<isize>,
    ) {
        self.normalize_current_line_patch_json_if_known();
        let mml = self.lines[self.cursor].trim().to_string();
        if !mml.is_empty() {
            self.record_notepad_history(&mml);
            self.record_patch_phrase_history(&mml);
            self.play_mml(mml);
            self.prefetch_normal_navigation_audio_cache(preferred_delta);
        }
    }

    pub(in crate::tui::input) fn insert_generated_line_above(&mut self) -> Result<(), String> {
        let patch_name = self.pick_random_patch_name()?;
        let mml = format!(
            "{} {}",
            Self::build_patch_json(&patch_name),
            crate::generate::pick_default_generate_phrase()
        );
        self.lines.insert(self.cursor, mml.clone());
        self.list_state.select(Some(self.cursor));
        self.record_notepad_history(&mml);
        self.record_patch_phrase_history(&mml);
        self.play_mml(mml);
        Ok(())
    }

    pub(in crate::tui::input) fn pick_random_patch_name(&mut self) -> Result<String, String> {
        self.pick_random_patch_name_with_query(None)?
            .ok_or_else(|| "patches_dirs にパッチが見つかりません".to_string())
    }

    pub(in crate::tui::input) fn pick_random_patch_name_with_query(
        &mut self,
        query: Option<&str>,
    ) -> Result<Option<String>, String> {
        if !crate::patches::has_configured_patch_dirs(&self.cfg) {
            return Err("patches_dirs が設定されていません".to_string());
        }
        let selection_query = query.map(str::trim).filter(|query| !query.is_empty());
        let candidates = {
            let state = self.patch_load_state.lock().unwrap();
            match &*state {
                PatchLoadState::Loading => return Err("パッチを読み込み中です...".to_string()),
                PatchLoadState::Err(e) => return Err(format!("パッチの読み込みに失敗: {}", e)),
                PatchLoadState::Ready(pairs) if pairs.is_empty() => {
                    return Err("patches_dirs にパッチが見つかりません".to_string());
                }
                PatchLoadState::Ready(pairs) => match selection_query {
                    Some(query) => filter_patches(pairs, query),
                    None => pairs.iter().map(|(display, _)| display.clone()).collect(),
                },
            }
        };
        let Some(index) = self
            .random_patch_decks
            .next_index(selection_query, candidates.len())
        else {
            return Ok(None);
        };
        Ok(Some(candidates[index].clone()))
    }

    pub(in crate::tui) fn start_insert(&mut self) {
        self.textarea = crate::text_input::new_single_line_textarea(&self.lines[self.cursor]);
        self.mode = Mode::Insert;
    }

    pub(in crate::tui::input) fn insert_empty_line_and_start_insert(&mut self, index: usize) {
        self.lines.insert(index, String::new());
        self.cursor = index;
        self.list_state.select(Some(self.cursor));
        self.start_insert();
    }

    pub(in crate::tui::input) fn delete_current_line(&mut self) {
        self.yank_buffer = Some(self.lines.remove(self.cursor));
        if self.lines.is_empty() {
            self.lines.push(String::new());
            self.cursor = 0;
        } else if self.cursor >= self.lines.len() {
            self.cursor = self.lines.len().saturating_sub(1);
        }
        self.list_state.select(Some(self.cursor));
    }

    pub(in crate::tui::input) fn paste_yanked_line(&mut self, insert_above: bool) -> bool {
        let Some(yanked) = self.yank_buffer.as_ref() else {
            return false;
        };
        let insert_at = if insert_above {
            self.cursor
        } else {
            self.cursor + 1
        };
        self.lines.insert(insert_at, yanked.clone());
        self.cursor = insert_at;
        self.list_state.select(Some(self.cursor));
        true
    }

    pub(in crate::tui::input) fn start_patch_phrase_for_current_line(&mut self) {
        self.start_patch_phrase_for_patch_name(self.current_line_patch_name());
    }

    pub(in crate::tui) fn current_line_patch_name(&self) -> Option<String> {
        self.lines
            .get(self.cursor)
            .and_then(|line| Self::extract_patch_phrase(line))
            .map(|(patch_name, _)| patch_name)
            .map(|patch_name| {
                self.resolve_loaded_patch_name(&patch_name)
                    .unwrap_or(patch_name)
            })
    }

    pub(in crate::tui) fn start_patch_phrase_for_patch_name(&mut self, patch_name: Option<String>) {
        match patch_name {
            Some(patch_name) => self.start_patch_phrase(patch_name),
            None => {
                *self.play_state.lock().unwrap() =
                    PlayState::Err("patch name JSON が見つかりません".to_string());
            }
        }
    }

    pub(in crate::tui) fn start_patch_select(&mut self) {
        self.start_patch_select_with_initial_patch_name(None);
    }

    pub(in crate::tui) fn open_patch_select_overlay(&mut self, initial_patch_name: Option<&str>) {
        if !crate::patches::has_configured_patch_dirs(&self.cfg) {
            *self.play_state.lock().unwrap() =
                PlayState::Err("patches_dirs が設定されていません".to_string());
            return;
        }

        let action = {
            let state = self.patch_load_state.lock().unwrap();
            match &*state {
                PatchLoadState::Loading => Err("パッチを読み込み中です...".to_string()),
                PatchLoadState::Err(e) => Err(format!("パッチの読み込みに失敗: {}", e)),
                PatchLoadState::Ready(p) if p.is_empty() => {
                    Err("patches_dirs にパッチが見つかりません".to_string())
                }
                PatchLoadState::Ready(_) => Ok(()),
            }
        };

        match action {
            Err(msg) => *self.play_state.lock().unwrap() = PlayState::Err(msg),
            Ok(()) => match initial_patch_name {
                Some(patch_name) => {
                    self.start_patch_select_with_initial_patch_name(Some(patch_name))
                }
                None => self.start_patch_select(),
            },
        }
    }

    pub(in crate::tui::input) fn set_empty_yank_error(&mut self) {
        *self.play_state.lock().unwrap() = PlayState::Err("yank バッファが空です".to_string());
    }
}
