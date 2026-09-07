//! loop tree の `/` 絞り込み。
//!
//! 条件のマッチ規則そのものは `cmrt_tui_core::text_filter`（画面横断の単一ソース）に任せ、
//! ここは「ツリーのどのノードを残すか」だけを持つ。
//!
//! 残す規則:
//! - wav 行: root からの相対パス全体（`/` 区切り）が条件にマッチしたら残す
//! - ディレクトリ行: 自分自身のパスがマッチしたらサブツリーごと全部残す。
//!   そうでなければ、子孫に残る wav があるときだけ残す
//!
//! 絞り込み中は `LoopBrowser::expanded` を書き換えず、残ったディレクトリを
//! 全部展開したものとして描く（[`crate::tree::ExpandedNodes::All`]）。
//! そのため絞り込みを解除すると、絞り込み前の折り畳み状態がそのまま戻る。

use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;
use regex::Regex;

use cmrt_tui_core::text_filter::{compile_condition, matches_any_field};
use cmrt_tui_core::text_input::{
    apply_key_event_to_textarea, new_single_line_textarea, textarea_value,
};

use super::{LoopBrowser, LoopBrowserAction, LoopWavId, TreeNode};

/// `/` を押してから Enter / Esc で抜けるまでの入力状態。
///
/// Esc は「絞り込みの解除」ではなく「`/` を押す前へ戻す」なので、
/// 戻し先のクエリと選択パスをここに控える。
pub(crate) struct FilterInput {
    textarea: TextArea<'static>,
    /// `/` を押したときの確定済みクエリ。Esc の戻し先。
    query_before: String,
    /// `/` を押したときに選んでいた行のパス。Esc の戻し先。
    path_before: Option<PathBuf>,
}

impl LoopBrowser {
    /// 確定済みの絞り込みクエリ。空なら絞り込みなし。
    pub fn filter_query(&self) -> &str {
        &self.filter_query
    }

    /// 絞り込み中か（＝可視ノードがクエリで削られているか）。
    ///
    /// クエリが不正な正規表現のときは、直前の有効な条件で絞り込まれたままなので true。
    pub fn filter_active(&self) -> bool {
        !self.filter_condition.is_empty()
    }

    /// クエリを差し替えて可視ノードを組み直す。
    ///
    /// 不正な正規表現（打鍵の途中の `(` など）はクエリだけ更新し、結果は直前の有効な
    /// 条件のまま保つ。カーソルは直前に選択していたパスが残っていればそれを維持し、
    /// 無ければ先頭へ。
    pub fn set_filter_query(&mut self, query: &str) {
        if self.filter_query == query {
            return;
        }
        let started_at = Instant::now();
        self.filter_query = query.to_string();
        let Ok(condition) = compile_condition(query) else {
            crate::performance::log_filter_query(
                started_at.elapsed(),
                query,
                self.visible.len(),
                self.wav_analyses.len(),
                "invalid-condition",
            );
            return;
        };
        self.filter_condition = condition;
        let selected = self.visible.get(self.cursor).map(|node| node.path.clone());
        self.rebuild_visible(None);
        self.cursor = selected
            .and_then(|path| self.visible.iter().position(|node| node.path == path))
            .unwrap_or(0);
        self.tree_scroll = self.tree_scroll.min(self.cursor);
        crate::performance::log_filter_query(
            started_at.elapsed(),
            query,
            self.visible.len(),
            self.wav_analyses.len(),
            if self.filter_condition.is_empty() {
                "released"
            } else {
                "filtered"
            },
        );
    }

    /// `/` の絞り込み入力中か。
    ///
    /// 入力中は上下移動もプレビュー再生も効かず、すべてのキーが入力欄へ入る。
    pub fn filter_input_active(&self) -> bool {
        self.filter_input.is_some()
    }

    /// 入力中の入力欄。描画（枠つき textarea）と、カーソル表示の判定に使う。
    pub fn filter_textarea(&self) -> Option<&TextArea<'static>> {
        self.filter_input.as_ref().map(|input| &input.textarea)
    }

    /// `/` で絞り込み入力を開く。
    ///
    /// 入力欄の初期値は確定済みのクエリ。全消し → Enter が解除の操作になる
    /// （既存 3 箇所の絞り込みと同じ）。
    pub(crate) fn open_filter_input(&mut self) {
        self.navigation_count.clear();
        let mut textarea = new_single_line_textarea(&self.filter_query);
        textarea.move_cursor(ratatui_textarea::CursorMove::End);
        self.filter_input = Some(FilterInput {
            textarea,
            query_before: self.filter_query.clone(),
            path_before: self.visible.get(self.cursor).map(|node| node.path.clone()),
        });
    }

    /// 絞り込み入力中のキー。自前で拾うのは Enter（確定）と Esc（巻き戻し）だけで、
    /// 残りはすべて入力欄へ渡す（`?` も `q` も文字として入る）。
    pub(crate) fn handle_filter_input_key(&mut self, key: KeyEvent) -> LoopBrowserAction {
        if self.filter_input.is_none() {
            return LoopBrowserAction::Continue;
        }
        if is_commit_key(key) {
            self.filter_input = None;
            return LoopBrowserAction::Continue;
        }
        if key.code == KeyCode::Esc {
            if let Some(input) = self.filter_input.take() {
                self.set_filter_query(&input.query_before);
                self.rebuild_visible_for_path(input.path_before.as_deref());
            }
            return LoopBrowserAction::Continue;
        }
        let Some(input) = self.filter_input.as_mut() else {
            return LoopBrowserAction::Continue;
        };
        if apply_key_event_to_textarea(&mut input.textarea, key) {
            let query = textarea_value(&input.textarea);
            self.set_filter_query(&query);
        }
        LoopBrowserAction::Continue
    }

    /// 絞り込みで、この wav が可視ノードから消えているか。
    ///
    /// 判定は [`retain_matching`] の裏返し。wav 自身の相対パスか、その祖先ディレクトリ
    /// （root 自身＝空文字列を含む）のどれかが条件にマッチしていれば残っている。
    pub(crate) fn filter_hides_wav(&self, wav: &LoopWavId) -> bool {
        if self.filter_condition.is_empty() {
            return false;
        }
        let mut relative = String::new();
        if matches_any_field(&self.filter_condition, &[relative.as_str()]) {
            return false;
        }
        for component in Path::new(&wav.relative).components() {
            let Component::Normal(name) = component else {
                continue;
            };
            relative = join_relative(&relative, &name.to_string_lossy());
            if matches_any_field(&self.filter_condition, &[relative.as_str()]) {
                return false;
            }
        }
        true
    }

    /// 絞り込み後のツリー。絞り込みなしのときは `None`（元の `roots` をそのまま使う合図）。
    pub(crate) fn filtered_roots(&self) -> Option<Vec<(PathBuf, TreeNode)>> {
        if self.filter_condition.is_empty() {
            return None;
        }
        Some(
            self.roots
                .iter()
                .filter_map(|(root_path, root)| {
                    retain_matching(root, "", &self.filter_condition)
                        .map(|node| (root_path.clone(), node))
                })
                .collect(),
        )
    }
}

/// `relative` は root からこのノードまでの `/` 区切りの相対パス（root 自身は空文字列）。
fn retain_matching(node: &TreeNode, relative: &str, condition: &[Regex]) -> Option<TreeNode> {
    if matches_any_field(condition, &[relative]) {
        // 自分自身がマッチしたらサブツリーごと残す。
        return Some(node.clone());
    }
    if node.is_wav {
        return None;
    }
    let children = node
        .children
        .iter()
        .filter_map(|child| {
            retain_matching(child, &join_relative(relative, &child.name), condition)
        })
        .collect::<Vec<_>>();
    if children.is_empty() {
        return None;
    }
    Some(TreeNode {
        children,
        ..node.clone()
    })
}

fn join_relative(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

/// 1 行入力欄の確定キー。
///
/// 端末では `Ctrl+M` が `Enter` と同じバイトで届くことがあるが、crossterm は
/// `Ctrl+M` として渡してくることもあるので両方拾う（グローバルの 1 行入力欄の作法）。
fn is_commit_key(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => true,
        KeyCode::Char('m') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}
