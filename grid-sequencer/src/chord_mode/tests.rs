use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use cmrt_chord::ChordProgressionCatalog;
use cmrt_realtime_play::PatchVoicing;

use super::*;

use crate::tests::ctx_with;
use crate::{GridPatchLoad, GridVoicingLookup, NoVoicingLookup};

mod cycle;
mod drum;
mod patch_role;
mod restore;

const CATALOG_JSON: &str = r#"[
    {"degrees":"I-IV-V-I","description":"test"},
    {"degrees":"IIm-V-I","description":"test"}
]"#;

fn catalog() -> ChordProgressionCatalog {
    ChordProgressionCatalog::from_json(CATALOG_JSON).unwrap()
}

fn patches() -> Vec<(String, String)> {
    vec![
        ("Keys/Poly.fxp".to_string(), "keys/poly.fxp".to_string()),
        ("Leads/Mono.fxp".to_string(), "leads/mono.fxp".to_string()),
    ]
}

/// `Keys/Poly.fxp` だけを poly と判定する lookup。
struct OnePolyPatch;

impl GridVoicingLookup for OnePolyPatch {
    fn cached_voicing(&self, patch: &str) -> Option<PatchVoicing> {
        match patch {
            "Keys/Poly.fxp" => Some(PatchVoicing::Poly),
            "Leads/Mono.fxp" => Some(PatchVoicing::Mono),
            _ => None,
        }
    }
}

/// voicing 判定がすべて未確定の lookup。
struct AllUnknown;

impl GridVoicingLookup for AllUnknown {
    fn cached_voicing(&self, _patch: &str) -> Option<PatchVoicing> {
        None
    }
}

fn press_c() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)
}

fn screen() -> GridSequencerScreen {
    GridSequencerScreen::new(None)
}

#[test]
fn c_turns_the_chord_mode_on_with_a_poly_patch() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);

    screen.handle_key(press_c(), now, &ctx);

    let chord = screen.state.chord().expect("chord mode が on になる");
    assert!(cmrt_chord::KEYS.contains(&chord.key()));
    assert!(chord.chord_count() >= 3);
    assert_eq!(
        screen.state.rows()[CHORD_ROW].patch.as_deref(),
        Some("Keys/Poly.fxp"),
        "和音の行には poly と判明した patch だけを当てる"
    );
    assert!(screen.chord_error().is_none());
}

#[test]
fn c_toggles_the_chord_mode_off_again() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);

    screen.handle_key(press_c(), now, &ctx);

    assert!(screen.state.chord().is_none());
    assert!(screen.chord_error().is_none());
}

#[test]
fn chord_mode_is_refused_when_no_patch_is_known_to_be_poly() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &AllUnknown);
    let mut screen = screen();
    screen.start(now, &ctx);

    screen.handle_key(press_c(), now, &ctx);

    assert!(screen.state.chord().is_none());
    assert_eq!(screen.chord_error(), Some(CHORD_PATCH_UNAVAILABLE));
}

#[test]
fn chord_mode_is_refused_while_the_patch_list_is_loading() {
    let now = Instant::now();
    let catalog = catalog();
    let ctx = ctx_with(GridPatchLoad::Loading, &catalog, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);

    screen.handle_key(press_c(), now, &ctx);

    assert!(screen.state.chord().is_none());
    assert_eq!(screen.chord_error(), Some(PATCHES_UNAVAILABLE));
}

#[test]
fn chord_mode_is_refused_without_a_progression_catalog() {
    let now = Instant::now();
    let empty = ChordProgressionCatalog::default();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &empty, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);

    screen.handle_key(press_c(), now, &ctx);

    assert!(screen.state.chord().is_none());
    assert_eq!(screen.chord_error(), Some(CATALOG_UNAVAILABLE));
}

#[test]
fn r_rerolls_the_progression_and_repicks_a_poly_patch() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);

    screen.handle_key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
        now,
        &ctx,
    );

    assert!(screen.state.chord().is_some(), "chord mode は維持される");
    assert_eq!(
        screen.state.rows()[CHORD_ROW].patch.as_deref(),
        Some("Keys/Poly.fxp"),
        "無差別抽選で mono を引いたあとも poly へ当て直す"
    );
}

#[test]
fn shift_r_keeps_the_chord_patch_but_rerolls_the_progression() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);
    screen.state.rows_mut()[CHORD_ROW].patch = Some("Kept/Patch.fxp".to_string());

    screen.handle_key(
        KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT),
        now,
        &ctx,
    );

    assert!(screen.state.chord().is_some());
    assert_eq!(
        screen.state.rows()[CHORD_ROW].patch.as_deref(),
        Some("Kept/Patch.fxp"),
        "R は音色ロードを避けるため patch を触らない"
    );
}

#[test]
fn no_voicing_lookup_never_reports_poly() {
    assert!(NoVoicingLookup.cached_voicing("Keys/Poly.fxp").is_none());
}

/// カテゴリ検証用。poly だが別カテゴリの patch を混ぜた一覧。
fn categorized_patches() -> Vec<(String, String)> {
    vec![
        (
            "patches_factory/Keys/Grand.fxp".to_string(),
            "patches_factory/keys/grand.fxp".to_string(),
        ),
        (
            "patches_factory/Pads/Warm.fxp".to_string(),
            "patches_factory/pads/warm.fxp".to_string(),
        ),
        (
            "patches_factory/Leads/Saw.fxp".to_string(),
            "patches_factory/leads/saw.fxp".to_string(),
        ),
        (
            "patches_factory/Basses/Sub.fxp".to_string(),
            "patches_factory/basses/sub.fxp".to_string(),
        ),
        (
            "patches_factory/Synth/Other.fxp".to_string(),
            "patches_factory/synth/other.fxp".to_string(),
        ),
        (
            "patches_factory/Drums/Perc Shaker.fxp".to_string(),
            "patches_factory/drums/perc shaker.fxp".to_string(),
        ),
        (
            "patches_factory/Drums/Closed Hat.fxp".to_string(),
            "patches_factory/drums/closed hat.fxp".to_string(),
        ),
        (
            "patches_factory/Drums/Snare.fxp".to_string(),
            "patches_factory/drums/snare.fxp".to_string(),
        ),
        (
            "patches_factory/Drums/Kick.fxp".to_string(),
            "patches_factory/drums/kick.fxp".to_string(),
        ),
        (
            "patches_3rdparty/Vendor/Organs/Drawbar.fxp".to_string(),
            "patches_3rdparty/vendor/organs/drawbar.fxp".to_string(),
        ),
    ]
}

/// すべて poly とみなす lookup。カテゴリだけを検証したいときに使う。
struct AllPoly;

impl GridVoicingLookup for AllPoly {
    fn cached_voicing(&self, _patch: &str) -> Option<PatchVoicing> {
        Some(PatchVoicing::Poly)
    }
}

#[test]
fn only_note_random_chord_mode_boosts_the_chord_row() {
    // 返すのは bank 2 本ぶん。差し替え先の bank にも同じ音量差を載せておかないと、
    // 切り替わった瞬間に和音だけ音量が落ちる。
    assert_eq!(crate::chord_gains_db(4, false, true), vec![0.0; 8]);
    assert_eq!(
        crate::chord_gains_db(4, true, false),
        vec![0.0; 8],
        "譜面を据え置く間は全instanceを同じゲインへ戻す"
    );
    assert_eq!(
        crate::chord_gains_db(4, true, true),
        vec![
            crate::CHORD_GAIN_DB,
            0.0,
            0.0,
            0.0,
            crate::CHORD_GAIN_DB,
            0.0,
            0.0,
            0.0
        ],
        "どちらの bank でも和音の行だけが持ち上がる"
    );
    assert_eq!(crate::CHORD_GAIN_DB, 6.0);
}

/// 抽選した進行は必ず auto voicing を通る。mode 切り替えは無く、常に効く。
#[test]
fn a_picked_progression_always_comes_back_auto_voiced() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);

    screen.handle_key(press_c(), now, &ctx);

    let chord = screen.state.chord().expect("chord mode が on になる");
    let voicings = (0..chord.chord_count())
        .map(|index| chord.voicing_at(index).expect("voicing がある"))
        .collect::<Vec<_>>();
    assert!(
        voicings.iter().all(|voicing| voicing.bass.is_some()),
        "bass 行が鳴らす音が付いている: {voicings:?}"
    );
    let (top_jump, _) = cmrt_chord::max_jumps(&voicings);
    assert!(
        top_jump <= 4,
        "top note の跳躍が縮んでいない: {top_jump} ({voicings:?})"
    );
}

/// 次サイクルは「いま鳴っている進行の最後のコード」に接続する。
#[test]
fn the_next_cycle_connects_to_the_last_chord_of_the_current_progression() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = patches();
    let ctx = ctx_with(GridPatchLoad::Ready(&patches), &catalog, &OnePolyPatch);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);
    let last = screen
        .state
        .chord()
        .expect("chord mode が on")
        .last_voicing()
        .expect("voicing がある");

    assert!(screen.stage_next_cycle(now, &ctx));

    let staged = screen
        .state
        .pending_chord_for_test()
        .expect("次サイクルが預けられている")
        .voicing_at(0)
        .expect("voicing がある");
    let bridge = vec![last, staged];
    let (top_jump, _) = cmrt_chord::max_jumps(&bridge);
    assert!(
        top_jump <= 6,
        "cycle 境界で top note が跳んでいる: {top_jump} ({bridge:?})"
    );
}
