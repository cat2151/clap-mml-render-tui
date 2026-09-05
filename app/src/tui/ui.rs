//! 画面の振り分け描画。
//!
//! 各画面の描画本体は画面ごとの crate（notepad / keyboard / loop browser /
//! grid sequencer）にあり、ここは `active_screen` に応じてどれを呼ぶかを決めるだけの層。

use ratatui::Frame;

// grid sequencer の描画は `cmrt-grid-sequencer` crate の `ui` モジュールにある。
use cmrt_grid_sequencer::ui as grid_sequencer;
// keyboard の描画は `cmrt-keyboard` crate の `ui` モジュールへ切り出した。
use cmrt_keyboard::ui as keyboard;
// loop browser の描画は `cmrt-loop-browser` crate の `ui` モジュールへ切り出した。
pub(in crate::tui) use cmrt_loop_browser::ui as loop_browser;
// notepad の描画は `cmrt-notepad` crate の `ui` モジュールへ切り出した。
use cmrt_notepad::ui as notepad;

use super::{PrimaryScreen, TuiApp};

pub(super) fn draw(app: &mut TuiApp<'_>, f: &mut Frame) {
    cmrt_tui_core::ui::draw_frame_background(f);
    match app.active_screen {
        PrimaryScreen::Keyboard => {
            app.sync_keyboard_patch_catalog();
            app.sync_keyboard_voicing_detection();
            let connection = app.keyboard_connection_status();
            keyboard::draw(&mut app.keyboard, &connection, f);
        }
        PrimaryScreen::LoopBrowser => {
            let play_state = app.playback_session.play_state_snapshot();
            loop_browser::draw(&mut app.loop_browser.state, &play_state, f);
        }
        PrimaryScreen::GridSequencer => {
            let connection = app.grid_sequencer_connection_status();
            grid_sequencer::draw(&app.grid_sequencer, &connection, f);
        }
        // DAW 画面は `DawApp` が自前の描画ループを持つ。ここへ来るのは
        // DAW から戻る途中の一瞬だけなので notepad として描く。
        PrimaryScreen::Notepad | PrimaryScreen::DailyDaw | PrimaryScreen::Daw => {
            notepad::draw(&mut app.notepad, f)
        }
    }

    if app.screen_switch_menu.is_open() {
        crate::screen_switch::draw_screen_switch_menu(f, app.active_screen);
    }
    if app.mml_overlay.is_open() {
        let sender_status = app
            .mml_overlay_sender
            .as_ref()
            .map(cmrt_mml_overlay::MmlOverlaySender::status)
            .unwrap_or_default();
        cmrt_mml_overlay::ui::draw_with_status(&app.mml_overlay, &sender_status, f);
    }
    // 通常運転ではない play server を掴んでいるときだけ右上に出る。DAW 画面は
    // 自前の描画ループを持つので、あちらでも同じものを呼んでいる（ADR 0017）。
    cmrt_tui_core::server_profile_badge::draw(f, app.play_server.server_binary());
    // 音が鳴らない理由なので、どの画面・どのオーバーレイよりも前に出す。
    if let Some(failure) = app.play_server_notice() {
        super::play_server_notice::draw(f, &failure);
    }
}

// 描画結果を読むヘルパ（`render_lines`）を別のテストモジュールからも引くので pub。
#[cfg(test)]
pub(in crate::tui) mod tests;
