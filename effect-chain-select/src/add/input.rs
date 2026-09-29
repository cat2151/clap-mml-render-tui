//! 追加・差し替え overlay のキー処理。

use cmrt_core::AudioEffectCatalog;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{clamped_index, EffectAddPane, EffectChainEditor, PAGE_STEP};

/// 追加・差し替え overlay のキーが host に求めること。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddKeyAction {
    None,
    /// chain 一覧へ戻ってほしい（chain は変えていない）。
    Back,
    Help,
    /// list カーソルの候補を入れた chain（[`EffectChainEditor::candidate_chain`]）を鳴らしてほしい。
    /// `preferred_delta` は直前のカーソル移動の向き（先読みの偏らせ方に使える）。
    Preview {
        preferred_delta: Option<isize>,
    },
    /// 候補の段だけ bypass した chain を鳴らしてほしい（効き具合を比べる基準の音）。
    PreviewBypassed,
    /// 候補を chain へ入れた。chain 一覧へ戻ってほしい。
    Committed,
}

impl EffectChainEditor {
    /// category を動かした後は kind pane を、kind を動かした後は list を組み直し、
    /// いずれも list カーソルは 0 へ戻る。list が 0 件の `Enter` は何もしない。
    /// `/` はどの pane に focus していても list の絞り込みを開始する。
    ///
    /// 候補（list カーソルの preset）が変わったときだけ [`AddKeyAction::Preview`] を返す。
    /// `h`/`l` は候補を変えないので鳴らし直させない。
    pub fn handle_add_key(
        &mut self,
        catalog: Option<&AudioEffectCatalog>,
        key: KeyEvent,
    ) -> AddKeyAction {
        if self.add.filter_active {
            return self.handle_add_filter_key(catalog, key);
        }

        let add = &mut self.add;
        match key.code {
            KeyCode::Esc => return AddKeyAction::Back,
            KeyCode::Char('?') => return AddKeyAction::Help,
            KeyCode::Char('h') | KeyCode::Left => {
                add.focus = add.focus.prev();
                return AddKeyAction::None;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                add.focus = add.focus.next();
                return AddKeyAction::None;
            }
            KeyCode::Char('/') => {
                add.begin_filter();
                return AddKeyAction::None;
            }
            KeyCode::Char(' ') => {
                return AddKeyAction::Preview {
                    preferred_delta: None,
                }
            }
            KeyCode::Char('b') => return AddKeyAction::PreviewBypassed,
            KeyCode::Char('r') => {
                return if add.random_jump_list() {
                    AddKeyAction::Preview {
                        preferred_delta: None,
                    }
                } else {
                    AddKeyAction::None
                };
            }
            KeyCode::Enter => return self.commit_candidate(catalog),
            _ => {}
        }

        let delta = match key.code {
            KeyCode::Char('j') | KeyCode::Down => 1,
            KeyCode::Char('k') | KeyCode::Up => -1,
            KeyCode::PageDown => PAGE_STEP,
            KeyCode::PageUp => -PAGE_STEP,
            KeyCode::Home => isize::MIN,
            KeyCode::End => isize::MAX,
            _ => return AddKeyAction::None,
        };
        let candidate_before = add.candidate();
        match add.focus {
            EffectAddPane::Categories => {
                add.category_cursor =
                    clamped_index(add.category_cursor, delta, add.categories.len());
                if let Some(catalog) = catalog {
                    add.rebuild_kinds(catalog);
                }
            }
            EffectAddPane::Kinds => {
                add.kind_cursor = clamped_index(add.kind_cursor, delta, add.kinds.len());
                if let Some(catalog) = catalog {
                    add.rebuild_list(catalog);
                }
            }
            EffectAddPane::List => {
                add.list_cursor = clamped_index(add.list_cursor, delta, add.list.len());
            }
        }
        preview_if_changed(candidate_before, add.candidate(), Some(delta))
    }

    fn commit_candidate(&mut self, catalog: Option<&AudioEffectCatalog>) -> AddKeyAction {
        if self.add.candidate().is_none() {
            return AddKeyAction::None;
        }
        if let Some(stage) = self.add.preset_stage(catalog, self.add.list_cursor) {
            match self.add.replace_target {
                Some(index) => self.replace_stage(index, stage),
                None => self.push_stage(stage),
            }
        }
        AddKeyAction::Committed
    }

    /// list の絞り込み編集中のキー処理。`Esc` で編集前の query へ戻す、`Enter` で確定、
    /// それ以外は textarea へ渡して list を更新する（他のキーは selector に渡さない）。
    fn handle_add_filter_key(
        &mut self,
        catalog: Option<&AudioEffectCatalog>,
        key: KeyEvent,
    ) -> AddKeyAction {
        let add = &mut self.add;
        cmrt_tui_core::text_input::sync_single_line_textarea(&mut add.query_textarea, &add.query);
        match key.code {
            KeyCode::Esc => {
                match catalog {
                    Some(catalog) => add.cancel_filter(catalog),
                    None => add.filter_active = false,
                }
                AddKeyAction::None
            }
            KeyCode::Enter => {
                add.filter_active = false;
                AddKeyAction::None
            }
            _ => {
                if !cmrt_tui_core::text_input::apply_key_event_to_textarea(
                    &mut add.query_textarea,
                    key,
                ) {
                    return AddKeyAction::None;
                }
                add.query = cmrt_tui_core::text_input::textarea_value(&add.query_textarea);
                let candidate_before = add.candidate();
                if let Some(catalog) = catalog {
                    add.rebuild_list(catalog);
                }
                preview_if_changed(candidate_before, add.candidate(), None)
            }
        }
    }
}

fn preview_if_changed(
    before: Option<usize>,
    after: Option<usize>,
    preferred_delta: Option<isize>,
) -> AddKeyAction {
    if before == after {
        AddKeyAction::None
    } else {
        AddKeyAction::Preview { preferred_delta }
    }
}
