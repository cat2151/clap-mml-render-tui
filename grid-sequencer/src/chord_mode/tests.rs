use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use cmrt_chord::ChordProgressionCatalog;
use cmrt_realtime_play::PatchVoicing;

use super::*;
use crate::tests::ctx_with;
use crate::{GridPatchLoad, GridVoicingLookup, NoVoicingLookup};

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

fn ctx_with_categories<'a>(
    patches: &'a [(String, String)],
    catalog: &'a ChordProgressionCatalog,
    voicing: &'a dyn GridVoicingLookup,
    categories: &'a [String],
) -> GridSequencerContext<'a> {
    let mut ctx = ctx_with(GridPatchLoad::Ready(patches), catalog, voicing);
    ctx.chord_patch_categories = categories;
    ctx
}

#[test]
fn the_chord_patch_is_limited_to_the_configured_categories() {
    let catalog = catalog();
    let patches = categorized_patches();
    let categories = ["Keys".to_string(), "Organs".to_string()];
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &categories);

    // 抽選なので、何回引いても対象カテゴリから出ないことを確かめる。
    for _ in 0..40 {
        let picked =
            crate::pick_chord_patch(ctx.patches(), ctx.voicing, ctx.chord_patch_categories)
                .expect("Keys / Organs は候補にある");
        assert!(
            picked.contains("/Keys/") || picked.contains("/Organs/"),
            "対象外のカテゴリを引いた: {picked}"
        );
    }
}

#[test]
fn an_empty_category_list_means_no_category_filter() {
    let catalog = catalog();
    let patches = categorized_patches();
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &[]);

    let mut seen = std::collections::HashSet::new();
    for _ in 0..200 {
        seen.insert(
            crate::pick_chord_patch(ctx.patches(), ctx.voicing, ctx.chord_patch_categories)
                .unwrap(),
        );
    }
    assert_eq!(seen.len(), patches.len(), "全カテゴリが当たりになる");
}

#[test]
fn a_category_with_no_poly_patch_yields_nothing() {
    let catalog = catalog();
    let patches = categorized_patches();
    let categories = ["Basses".to_string()];
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &categories);
    let mut screen = screen();
    screen.start(Instant::now(), &ctx);

    screen.handle_key(press_c(), Instant::now(), &ctx);

    assert!(screen.state.chord().is_none());
    assert_eq!(screen.chord_error(), Some(CHORD_PATCH_UNAVAILABLE));
}

/// 進行を1周したら、進行・Key に加えて全行の音色も引き直す。
#[test]
fn staging_the_next_cycle_rerolls_every_patch() {
    let now = Instant::now();
    let catalog = catalog();
    let patches = categorized_patches();
    let categories = ["Keys".to_string(), "Organs".to_string()];
    let ctx = ctx_with_categories(&patches, &catalog, &AllPoly, &categories);
    let mut screen = screen();
    screen.start(now, &ctx);
    screen.handle_key(press_c(), now, &ctx);
    for row in screen.state.rows_mut() {
        row.patch = Some("Stale/Patch.fxp".to_string());
    }

    assert!(screen.stage_next_cycle(now, &ctx));

    // 鳴っている grid はそのまま。引き直しは差し替え待ちの側にだけ載る。
    assert!(
        screen
            .state
            .rows()
            .iter()
            .all(|row| row.patch.as_deref() == Some("Stale/Patch.fxp")),
        "演奏中の grid は触らない"
    );
    let staged = screen.state.pending_rows_for_test();
    assert!(
        staged
            .iter()
            .all(|row| row.patch.as_deref() != Some("Stale/Patch.fxp")),
        "全行の音色が引き直される"
    );
    let chord_patch = staged[CHORD_ROW].patch.clone().unwrap();
    assert!(
        chord_patch.contains("/Keys/") || chord_patch.contains("/Organs/"),
        "和音の行は対象カテゴリのまま: {chord_patch}"
    );
    assert!(screen.state.chord().is_some());
}

#[test]
fn only_the_chord_row_is_boosted_and_only_while_the_chord_mode_is_on() {
    // 返すのは bank 2 本ぶん。差し替え先の bank にも同じ音量差を載せておかないと、
    // 切り替わった瞬間に和音だけ音量が落ちる。
    assert_eq!(crate::chord_gains_db(4, false), vec![0.0; 8]);
    assert_eq!(
        crate::chord_gains_db(4, true),
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
