//! loop tree のカーソル移動と、ディレクトリの開閉。
//!
//! キーの割り当ては [`crate::input`]、可視ノードの組み立ては [`crate::tree`] が持つ。
//! ここは「今どの行を選んでいるか」を動かす操作だけを持つ。

use super::*;

impl LoopBrowser {
    pub(crate) fn move_cursor(&mut self, delta: isize) -> LoopBrowserAction {
        let started_at = Instant::now();
        if self.visible.is_empty() {
            return LoopBrowserAction::Continue;
        }
        let previous = self.cursor;
        let max = self.visible.len().saturating_sub(1) as isize;
        let next = (self.cursor as isize).saturating_add(delta).clamp(0, max) as usize;
        if next == self.cursor {
            return LoopBrowserAction::Continue;
        }
        self.cursor = next;
        let trace_id = super::performance::next_trace_id();
        self.pending_render_trace = Some(trace_id);
        let selected_is_wav = self.visible[next].is_wav;
        let action = self.selected_play_action_with_trace(trace_id);
        super::performance::log_cursor_move(
            trace_id,
            started_at.elapsed(),
            previous,
            next,
            self.visible.len(),
            if selected_is_wav { "wav" } else { "directory" },
            selected_is_wav,
        );
        action
    }

    pub(crate) fn expand_or_play(&mut self) -> LoopBrowserAction {
        let Some(node) = self.visible.get(self.cursor).cloned() else {
            return LoopBrowserAction::Continue;
        };
        if node.is_wav {
            let trace_id = super::performance::next_trace_id();
            self.pending_preview_trace = Some(trace_id);
            return LoopBrowserAction::Preview(node.path);
        }
        if self.expanded.insert(node.key.clone()) {
            self.rebuild_visible(Some(&node.key));
        }
        LoopBrowserAction::Continue
    }

    pub(crate) fn collapse_or_select_parent(&mut self) -> bool {
        let Some(node) = self.visible.get(self.cursor).cloned() else {
            return false;
        };
        if !node.is_wav && self.expanded.remove(&node.key) {
            self.rebuild_visible(Some(&node.key));
            return true;
        }
        if node.depth == 0 || node.key.components.is_empty() {
            return false;
        }
        let mut parent = node.key;
        parent.components.pop();
        self.rebuild_visible(Some(&parent));
        true
    }
}
