//! Guitar Articulation 画面のサンプル MID モードで、app が置き場の一覧を作り、file を読み、
//! 画面が求めた全体・1 音をどの messages・どの音色で `play_line` へ渡すか。
//!
//! MID は一時ディレクトリへ `midly` で書き、実 server の代わりに記録 sender を使う。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use super::guitar_articulation::{
    app_with_mml, flat_events, messages, plain, sent_messages, wait_until,
};
use super::*;
use crate::tui::guitar_articulation::{convert, RuleTable, PATCH};
use crate::tui::guitar_articulation_sample_midi::{sample_midi_dir_in, sample_midi_files};
use cmrt_mml_overlay::{LivePatch, RecordingSink};

const QUARTER: u16 = 480;

fn midi(delta: u32, message: MidiMessage) -> TrackEvent<'static> {
    TrackEvent {
        delta: delta.into(),
        kind: TrackEventKind::Midi {
            channel: 0.into(),
            message,
        },
    }
}

fn note(delta: u32, key: u8, vel: u8) -> TrackEvent<'static> {
    midi(
        delta,
        MidiMessage::NoteOn {
            key: key.into(),
            vel: vel.into(),
        },
    )
}

fn control(delta: u32, controller: u8, value: u8) -> TrackEvent<'static> {
    midi(
        delta,
        MidiMessage::Controller {
            controller: controller.into(),
            value: value.into(),
        },
    )
}

/// KS 17（Sus_Down）と CC20 を持つ、2 音（60・62）の MID。
fn sample_smf() -> Vec<u8> {
    let track = vec![
        control(0, 20, 64),
        note(0, 17, 100),
        note(0, 60, 100),
        note(u32::from(QUARTER), 60, 0),
        note(0, 17, 0),
        control(0, 20, 90),
        note(0, 62, 100),
        note(u32::from(QUARTER), 62, 0),
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ];
    let mut bytes = Vec::new();
    Smf {
        header: Header::new(Format::SingleTrack, Timing::Metrical(QUARTER.into())),
        tracks: vec![track],
    }
    .write_std(&mut bytes)
    .unwrap();
    bytes
}

/// `tag` つきの一時ディレクトリに `names` の file を書く。`.mid` 系には MID を、ほかは空を書く。
fn temp_dir_with(tag: &str, names: &[&str]) -> PathBuf {
    let dir = crate::test_utils::unique_test_dir(tag);
    std::fs::create_dir_all(&dir).unwrap();
    for name in names {
        let is_mid = Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mid"));
        let content = if is_mid { sample_smf() } else { Vec::new() };
        std::fs::write(dir.join(name), content).unwrap();
    }
    dir
}

#[test]
fn the_list_is_the_mid_files_by_name_whatever_the_extension_case() {
    let dir = temp_dir_with("ga_sample_midi_list", &["b.MID", "notes.txt", "a.mid"]);

    let files = sample_midi_files(&dir);

    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(files, Ok(vec![dir.join("a.mid"), dir.join("b.MID")]));
}

#[test]
fn a_missing_directory_names_its_path() {
    let dir = crate::test_utils::unique_test_dir("ga_sample_midi_missing");

    let error = sample_midi_files(&dir).unwrap_err();

    assert!(error.contains(&dir.display().to_string()), "{error}");
}

/// 一覧を開いて `Enter` で読み込み、MID モードに入った画面。読み込んだだけでは鳴らさない。
fn app_in_sample_midi<'a>(tag: &str) -> (TuiApp<'a>, Arc<RecordingSink>) {
    let (mut app, sink) = app_with_mml();
    let dir = temp_dir_with(tag, &["CC20.mid"]);
    let files = sample_midi_files(&dir);
    app.guitar_articulation.open_sample_midi_list(files);
    app.handle_guitar_articulation_key_event(plain(KeyCode::Enter));
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(
        app.guitar_articulation
            .sample_midi()
            .map(|midi| midi.name()),
        Some("CC20.mid")
    );
    assert_eq!(sink.timelines(), 0);
    (app, sink)
}

#[test]
fn space_sends_every_event_of_the_file() {
    let (mut app, sink) = app_in_sample_midi("ga_sample_midi_space");
    let all = cmrt_chord::timed_smf_events(&sample_smf()).unwrap().events;
    assert_eq!(app.guitar_articulation.sample_midi_events(None), all);

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char(' ')));

    wait_until("MID の全体", || {
        sink.timeline_events().len() >= all.len()
    });
    assert_eq!(sent_messages(&sink, 0), messages(&all));
    assert!(all.iter().any(|event| event.message[0] == 0xB0));
    assert_eq!(sink.timelines(), 1);
}

#[test]
fn l_sends_the_next_note_with_its_leading_state_without_the_note_mode() {
    let (mut app, sink) = app_in_sample_midi("ga_sample_midi_note");
    let note = app.guitar_articulation.sample_midi_events(Some(1));

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('l')));

    wait_until("2 音目", || sink.timeline_events().len() >= note.len());
    assert_eq!(sent_messages(&sink, 0), messages(&note));
    assert!(note.iter().any(|event| event.message == [0x90, 62, 100]));
    assert!(!note.iter().any(|event| event.message == [0x90, 60, 100]));
    assert_eq!(sink.timelines(), 1);
}

#[test]
fn j_in_the_list_previews_the_whole_file_without_entering_the_midi_mode() {
    let (mut app, sink) = app_with_mml();
    let dir = temp_dir_with("ga_sample_midi_preview", &["a.mid", "b.mid"]);
    app.guitar_articulation
        .open_sample_midi_list(sample_midi_files(&dir));
    let all = cmrt_chord::timed_smf_events(&sample_smf()).unwrap().events;

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('j')));

    std::fs::remove_dir_all(&dir).ok();
    wait_until("試聴", || sink.timeline_events().len() >= all.len());
    assert_eq!(sent_messages(&sink, 0), messages(&all));
    assert_eq!(sink.timelines(), 1);
    assert!(app.guitar_articulation.sample_midi().is_none());
    assert_eq!(app.guitar_articulation.sample_midi_list().unwrap().1, 1);
}

/// MID の演奏も MML の演奏と同じ音色・chain で送るので、sender は音色を読み直さない。
#[test]
fn the_file_is_sent_to_the_same_patch_as_the_mml() {
    let (mut app, sink) = app_with_mml();
    let whole = convert(&flat_events(), &RuleTable::default());
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char(' ')));
    wait_until("MML", || sink.timeline_events().len() >= whole.len());
    let dir = temp_dir_with("ga_sample_midi_patch", &["CC20.mid"]);
    app.guitar_articulation
        .open_sample_midi_list(sample_midi_files(&dir));
    app.handle_guitar_articulation_key_event(plain(KeyCode::Enter));
    std::fs::remove_dir_all(&dir).ok();
    let all = app.guitar_articulation.sample_midi_events(None);

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char(' ')));

    wait_until("MID", || {
        sink.timeline_events().len() >= whole.len() + all.len()
    });
    assert_eq!(sent_messages(&sink, whole.len()), messages(&all));
    assert_eq!(sink.timelines(), 2);
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some(PATCH))]);
}

#[test]
fn o_while_the_patches_are_loading_keeps_the_list_closed_and_says_why() {
    let (mut app, _sink) = app_with_mml();
    *app.patch_load_state.lock().unwrap() = PatchLoadState::Loading;

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('o')));

    assert!(app.guitar_articulation.sample_midi_list().is_none());
    assert_eq!(
        app.guitar_articulation.error.as_deref(),
        Some("音色の一覧を読み込み中です")
    );
}

#[test]
fn a_file_that_is_not_a_midi_file_keeps_the_list_open_with_the_reason() {
    let (mut app, sink) = app_with_mml();
    let dir = temp_dir_with("ga_sample_midi_broken", &[]);
    std::fs::write(dir.join("broken.mid"), b"not a midi file").unwrap();
    app.guitar_articulation
        .open_sample_midi_list(sample_midi_files(&dir));

    app.handle_guitar_articulation_key_event(plain(KeyCode::Enter));

    std::fs::remove_dir_all(&dir).ok();
    assert!(app.guitar_articulation.sample_midi().is_none());
    assert!(app.guitar_articulation.sample_midi_list().is_some());
    let error = app.guitar_articulation.error.clone().unwrap_or_default();
    assert!(error.starts_with("broken.mid: "), "{error}");
    assert_eq!(sink.timelines(), 0);
}

/// 付属の Control_Change の MID（METAL-GTX の配布物の file 名）。
const CONTROL_CHANGE_FILES: [&str; 7] = [
    "CC20_21_Vibrato_Control.mid",
    "CC22_Mute_Control.mid",
    "CC24_25_Release_Auto_Detection.mid",
    "CC24_Auto_Repetition.mid",
    "CC24_Auto_Slide_Out.mid",
    "CC26_Action_Slide.mid",
    "CC27_Interval_Control.mid",
];

/// 実機の Sforzando の置き場から、画面の音色と同じ規則で付属 MID の置き場を解決し、7 本が名前順に並び、
/// どれも CC を残して読めることを確かめる。
#[test]
#[ignore = "installed Sforzando is required: set CMRT_TEST_SFORZANDO_CLAP"]
fn the_installed_sforzando_lists_and_reads_the_seven_control_change_files() {
    let Some(clap) = std::env::var("CMRT_TEST_SFORZANDO_CLAP").ok() else {
        eprintln!("skip: CMRT_TEST_SFORZANDO_CLAP is not set");
        return;
    };
    let mut cfg: crate::config::Config =
        toml::from_str(&cmrt_runtime::default_config_content()).unwrap();
    cfg.plugins.insert(
        "Sforzando".to_string(),
        cmrt_runtime::PluginProfile {
            plugin_path: clap,
            plugin_id: Some(cmrt_runtime::SFORZANDO_PLUGIN_ID.to_string()),
            patches_dirs: None,
        },
    );
    cmrt_runtime::apply_primary_plugin_profile(&mut cfg).unwrap();
    let plugins = cmrt_tui_core::patch_plugins::PatchPlugins::from_config(&cfg);

    let dir = sample_midi_dir_in(&plugins).unwrap();
    let files = sample_midi_files(&dir).unwrap();

    let names: Vec<String> = files
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, CONTROL_CHANGE_FILES);
    for path in &files {
        let events = cmrt_chord::timed_smf_events(&std::fs::read(path).unwrap())
            .unwrap()
            .events;
        let controls = events
            .iter()
            .filter(|event| event.message[0] & 0xF0 == 0xB0)
            .count();
        eprintln!(
            "{}: events={} controls={controls}",
            path.display(),
            events.len()
        );
        assert!(controls > 0, "{}", path.display());
    }
}
