use crate::NotepadScreen;
use crate::{filter_patches_by_display_path, Mode, PatchLoadState, PlayState};

impl<'a> NotepadScreen<'a> {
    fn normalize_current_line_patch_json_if_known(&mut self) {
        let Some(current_patch_name) = self.current_line_patch_name() else {
            return;
        };
        let Some(raw_patch_name) = self
            .editor
            .lines
            .get(self.editor.cursor)
            .and_then(|line| Self::extract_patch_phrase(line))
            .map(|(patch_name, _)| patch_name)
        else {
            return;
        };
        if current_patch_name != raw_patch_name {
            self.replace_current_line_patch(&current_patch_name);
        }
    }

    pub(crate) fn play_mml(&mut self, mml: String) {
        #[cfg(any(test, feature = "test-support"))]
        if self.playback.render_queue.is_disabled() {
            // new_for_test() ではレンダリングキューが無効なため、
            // テスト中は再生スレッドを起動せず play_state 更新だけを検証する。
            *self.playback.session.play_state().lock().unwrap() = PlayState::Running(mml);
            return;
        }

        self.kick_play(mml);
    }

    fn prefetch_normal_navigation_audio_cache(&self, preferred_delta: Option<isize>) {
        self.prefetch_navigation_audio_cache(
            self.editor.cursor,
            self.editor.lines.len(),
            self.editor.page_size,
            preferred_delta,
            |index| {
                self.editor
                    .lines
                    .get(index)
                    .map(|line| line.trim().to_string())
                    .filter(|mml| !mml.is_empty())
            },
        );
    }

    pub(crate) fn prime_normal_mode_startup_cache(&self) {
        let Some(current_mml) = self
            .editor
            .lines
            .get(self.editor.cursor)
            .map(|line| line.trim().to_string())
            .filter(|mml| !mml.is_empty())
        else {
            return;
        };
        self.load_disk_cached_audio_into_memory_if_present(&current_mml);
        let navigation_targets = cmrt_tui_core::navigation::predicted_navigation_indices(
            self.editor.cursor,
            self.editor.lines.len(),
            self.editor.page_size,
        )
        .into_iter()
        .filter_map(|index| {
            self.editor
                .lines
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
    pub(crate) fn hydrate_all_lines_from_disk_cache_at_startup(&self) {
        for line in &self.editor.lines {
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
        if self.audio.cache.lock().unwrap().contains_key(mml) {
            return;
        }
        let Some(samples) =
            crate::disk_cache::load_valid_cached_wav(mml, self.cfg.sample_rate as u32)
        else {
            return;
        };
        let mut cache = self.audio.cache.lock().unwrap();
        let mut cache_order = self.audio.order.lock().unwrap();
        crate::cache::try_insert_cache(
            &mut cache,
            &mut cache_order,
            mml.to_string(),
            samples,
            false,
        );
    }

    pub(crate) fn play_current_line(&mut self) {
        self.play_current_line_with_navigation_hint(None);
    }

    pub(crate) fn play_current_line_with_navigation_hint(
        &mut self,
        preferred_delta: Option<isize>,
    ) {
        self.normalize_current_line_patch_json_if_known();
        let mml = self.editor.lines[self.editor.cursor].trim().to_string();
        if !mml.is_empty() {
            self.record_notepad_history(&mml);
            self.record_patch_phrase_history(&mml);
            self.play_mml(mml);
            self.prefetch_normal_navigation_audio_cache(preferred_delta);
        }
    }

    pub(crate) fn insert_generated_line_above(&mut self) -> Result<(), String> {
        let patch_name = self.pick_random_patch_name()?;
        let mml = format!(
            "{} {}",
            Self::build_patch_json(&patch_name),
            cmrt_tui_core::generate::pick_default_generate_phrase()
        );
        self.editor.lines.insert(self.editor.cursor, mml.clone());
        self.editor.list_state.select(Some(self.editor.cursor));
        self.record_notepad_history(&mml);
        self.record_patch_phrase_history(&mml);
        self.play_mml(mml);
        Ok(())
    }

    pub(crate) fn pick_random_patch_name(&mut self) -> Result<String, String> {
        self.pick_random_patch_name_with_query(None)?
            .ok_or_else(|| "patches_dirs にパッチが見つかりません".to_string())
    }

    pub(crate) fn pick_random_patch_name_with_query(
        &mut self,
        query: Option<&str>,
    ) -> Result<Option<String>, String> {
        if !cmrt_tui_core::patches::has_configured_patch_dirs(&self.cfg) {
            return Err("patches_dirs が設定されていません".to_string());
        }
        let selection_query = query.map(str::trim).filter(|query| !query.is_empty());
        let candidates = {
            let state = self.patch_load_state.lock().unwrap();
            match &*state {
                PatchLoadState::Loading => return Err("パッチを読み込み中です...".to_string()),
                PatchLoadState::Err(e) => return Err(format!("パッチの読み込みに失敗: {}", e)),
                PatchLoadState::Ready(snapshot) if snapshot.pairs().is_empty() => {
                    return Err("patches_dirs にパッチが見つかりません".to_string());
                }
                PatchLoadState::Ready(snapshot) => match selection_query {
                    Some(query) => filter_patches_by_display_path(snapshot.pairs(), query),
                    None => snapshot
                        .pairs()
                        .iter()
                        .map(|(display, _)| display.clone())
                        .collect(),
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

    pub(crate) fn start_insert(&mut self) {
        self.editor.textarea = cmrt_tui_core::text_input::new_single_line_textarea(
            &self.editor.lines[self.editor.cursor],
        );
        self.mode = Mode::Insert;
    }

    pub(crate) fn insert_empty_line_and_start_insert(&mut self, index: usize) {
        self.editor.lines.insert(index, String::new());
        self.editor.cursor = index;
        self.editor.list_state.select(Some(self.editor.cursor));
        self.start_insert();
    }

    pub(crate) fn delete_current_line(&mut self) {
        self.editor.yank_buffer = Some(self.editor.lines.remove(self.editor.cursor));
        if self.editor.lines.is_empty() {
            self.editor.lines.push(String::new());
            self.editor.cursor = 0;
        } else if self.editor.cursor >= self.editor.lines.len() {
            self.editor.cursor = self.editor.lines.len().saturating_sub(1);
        }
        self.editor.list_state.select(Some(self.editor.cursor));
    }

    pub(crate) fn paste_yanked_line(&mut self, insert_above: bool) -> bool {
        let Some(yanked) = self.editor.yank_buffer.as_ref() else {
            return false;
        };
        let insert_at = if insert_above {
            self.editor.cursor
        } else {
            self.editor.cursor + 1
        };
        self.editor.lines.insert(insert_at, yanked.clone());
        self.editor.cursor = insert_at;
        self.editor.list_state.select(Some(self.editor.cursor));
        true
    }

    pub(crate) fn start_patch_phrase_for_current_line(&mut self) {
        self.start_patch_phrase_for_patch_name(self.current_line_patch_name());
    }

    pub fn current_line_patch_name(&self) -> Option<String> {
        self.editor
            .lines
            .get(self.editor.cursor)
            .and_then(|line| Self::extract_patch_phrase(line))
            .map(|(patch_name, _)| patch_name)
            .map(|patch_name| {
                self.resolve_loaded_patch_name(&patch_name)
                    .unwrap_or(patch_name)
            })
    }

    pub(crate) fn start_patch_phrase_for_patch_name(&mut self, patch_name: Option<String>) {
        match patch_name {
            Some(patch_name) => self.start_patch_phrase(patch_name),
            None => {
                *self.playback.session.play_state().lock().unwrap() =
                    PlayState::Err("patch name JSON が見つかりません".to_string());
            }
        }
    }

    pub(crate) fn start_patch_select(&mut self) {
        self.start_patch_select_with_initial_patch_name(None);
    }

    pub(crate) fn open_patch_select_overlay(&mut self, initial_patch_name: Option<&str>) {
        if !cmrt_tui_core::patches::has_configured_patch_dirs(&self.cfg) {
            *self.playback.session.play_state().lock().unwrap() =
                PlayState::Err("patches_dirs が設定されていません".to_string());
            return;
        }

        let action = {
            let state = self.patch_load_state.lock().unwrap();
            match &*state {
                PatchLoadState::Loading => Err("パッチを読み込み中です...".to_string()),
                PatchLoadState::Err(e) => Err(format!("パッチの読み込みに失敗: {}", e)),
                PatchLoadState::Ready(snapshot) if snapshot.pairs().is_empty() => {
                    Err("patches_dirs にパッチが見つかりません".to_string())
                }
                PatchLoadState::Ready(_) => Ok(()),
            }
        };

        match action {
            Err(msg) => *self.playback.session.play_state().lock().unwrap() = PlayState::Err(msg),
            Ok(()) => match initial_patch_name {
                Some(patch_name) => {
                    self.start_patch_select_with_initial_patch_name(Some(patch_name))
                }
                None => self.start_patch_select(),
            },
        }
    }

    pub(crate) fn set_empty_yank_error(&mut self) {
        *self.playback.session.play_state().lock().unwrap() =
            PlayState::Err("yank バッファが空です".to_string());
    }
}
