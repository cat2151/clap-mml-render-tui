//! Catalogの排他的Roleを、Grid上の行用途へ割り当てる。

use std::collections::{HashMap, HashSet};

use cmrt_patches::{DrumPatchRole, PatchRole};
use cmrt_realtime_play::PatchVoicing;
use cmrt_rhythm::DrumRole;
use rand::seq::SliceRandom;

use crate::{
    patch_notice::{catalog_unavailable, PatchNotice, PatchUnavailable},
    GridInstance, GridSequencerContext, GridSequencerScreen, ARPEGGIO_ROW, BASS_ROW, CHORD_ROW,
};

/// Grid上の用途。catalogの分類語彙とは分けて、行が要求する能力を表す。
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum GridPatchPurpose {
    Note,
    Chord,
    Bass,
    Arpeggio,
    Kick,
    Snare,
    HiHat,
    Percussion,
}

/// chord modeとdrum割り当てから、その行の用途を決める。
pub fn row_patch_purpose(
    instance: usize,
    chord_on: bool,
    drum: Option<DrumRole>,
) -> GridPatchPurpose {
    match drum {
        Some(DrumRole::Kick) => GridPatchPurpose::Kick,
        Some(DrumRole::Snare) => GridPatchPurpose::Snare,
        Some(DrumRole::HiHat) => GridPatchPurpose::HiHat,
        Some(DrumRole::Percussion) => GridPatchPurpose::Percussion,
        None => match instance {
            CHORD_ROW if chord_on => GridPatchPurpose::Chord,
            BASS_ROW if chord_on => GridPatchPurpose::Bass,
            ARPEGGIO_ROW if chord_on => GridPatchPurpose::Arpeggio,
            _ => GridPatchPurpose::Note,
        },
    }
}

/// Grid用途の候補を、画面と診断CLIで共有する。
pub fn candidates_for_purpose<'a>(
    ctx: &'a GridSequencerContext<'a>,
    purpose: GridPatchPurpose,
    reserved_roles: &[PatchRole],
) -> Vec<&'a str> {
    match purpose {
        GridPatchPurpose::Note => ctx
            .patches()
            .iter()
            .filter(|(display, _)| {
                ctx.patch_roles
                    .role_of(display)
                    .is_none_or(|role| !reserved_roles.contains(&role))
            })
            .map(|(display, _)| display.as_str())
            .collect(),
        GridPatchPurpose::Chord => ctx
            .patch_roles
            .candidates(PatchRole::Chord)
            .iter()
            .filter(|patch| ctx.voicing.cached_voicing(patch) == Some(PatchVoicing::Poly))
            .map(String::as_str)
            .collect(),
        GridPatchPurpose::Bass => string_refs(ctx.patch_roles.candidates(PatchRole::Bass)),
        GridPatchPurpose::Arpeggio => string_refs(ctx.patch_roles.candidates(PatchRole::Lead)),
        GridPatchPurpose::Kick => string_refs(ctx.patch_roles.drum_candidates(DrumPatchRole::Kick)),
        GridPatchPurpose::Snare => {
            string_refs(ctx.patch_roles.drum_candidates(DrumPatchRole::Snare))
        }
        GridPatchPurpose::HiHat => {
            string_refs(ctx.patch_roles.drum_candidates(DrumPatchRole::HiHat))
        }
        GridPatchPurpose::Percussion => {
            string_refs(ctx.patch_roles.drum_candidates(DrumPatchRole::Percussion))
        }
    }
}

fn string_refs(values: &[String]) -> Vec<&str> {
    values.iter().map(String::as_str).collect()
}

impl GridSequencerScreen {
    pub(crate) fn row_patch_purpose(&self, row: usize) -> GridPatchPurpose {
        row_patch_purpose(row, self.state.chord().is_some(), self.state.drum_role(row))
    }

    /// 専用行が実際に存在するときだけ、そのcatalog Roleを通常NOTEから予約する。
    fn reserved_patch_roles(&self) -> HashSet<PatchRole> {
        let mut reserved = HashSet::new();
        let count = self.state.instance_count();
        if self.state.chord().is_some() {
            if CHORD_ROW < count {
                reserved.insert(PatchRole::Chord);
            }
            if BASS_ROW < count {
                reserved.insert(PatchRole::Bass);
            }
            if ARPEGGIO_ROW < count {
                reserved.insert(PatchRole::Lead);
            }
        }
        if (0..count).any(|row| self.state.drum_role(row).is_some()) {
            reserved.insert(PatchRole::Drum);
        }
        reserved
    }

    /// 行用途に合う候補。通常NOTEだけは、専用行が予約済みのRoleを除く全catalogを使う。
    pub(crate) fn patch_candidates_for_row(
        &self,
        row: usize,
        ctx: &GridSequencerContext<'_>,
    ) -> Vec<String> {
        self.patch_candidates_for_purpose(self.row_patch_purpose(row), ctx)
    }

    pub(crate) fn patch_candidates_for_purpose(
        &self,
        purpose: GridPatchPurpose,
        ctx: &GridSequencerContext<'_>,
    ) -> Vec<String> {
        let reserved = self.reserved_patch_roles().into_iter().collect::<Vec<_>>();
        candidates_for_purpose(ctx, purpose, &reserved)
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn draw_patch_assignments(
        &self,
        instances: &[GridInstance],
        ctx: &GridSequencerContext<'_>,
        only_missing: bool,
        scope: PatchAssignmentScope,
    ) -> (Vec<Option<String>>, bool) {
        let mut bags = HashMap::<GridPatchPurpose, PatchDrawBag>::new();
        let mut missing_pool = false;
        let assignments = instances
            .iter()
            .enumerate()
            .map(|(row, instance)| {
                if only_missing && instance.patch.is_some() {
                    return None;
                }
                let purpose = self.row_patch_purpose(row);
                if !scope.includes(purpose) {
                    return None;
                }
                let bag = bags
                    .entry(purpose)
                    .or_insert_with(|| PatchDrawBag::new(self.patch_candidates_for_row(row, ctx)));
                let patch = bag.draw();
                missing_pool |= patch.is_none();
                patch
            })
            .collect();
        (assignments, missing_pool)
    }

    /// 全行を用途別候補から直接抽選する。候補が空の行は現在値を保持する。
    pub(crate) fn apply_random_patches(
        &mut self,
        ctx: &GridSequencerContext<'_>,
        only_missing: bool,
    ) -> usize {
        let (assignments, missing_pool) = self.draw_patch_assignments(
            self.state.instances(),
            ctx,
            only_missing,
            PatchAssignmentScope::All,
        );
        let mut applied = 0;
        for (instance, patch) in self.state.instances_mut().iter_mut().zip(assignments) {
            if let Some(patch) = patch {
                instance.patch = Some(patch);
                applied += 1;
            }
        }
        self.show_missing_pool_notice(ctx, missing_pool);
        applied
    }

    /// chord modeを有効化した直後、検証済みChord行以外の専用行だけを抽選する。
    pub(crate) fn apply_dedicated_patches(&mut self, ctx: &GridSequencerContext<'_>) -> usize {
        let (assignments, missing_pool) = self.draw_patch_assignments(
            self.state.instances(),
            ctx,
            false,
            PatchAssignmentScope::DedicatedExceptChord,
        );
        let mut applied = 0;
        for (instance, patch) in self.state.instances_mut().iter_mut().zip(assignments) {
            if let Some(patch) = patch {
                instance.patch = Some(patch);
                applied += 1;
            }
        }
        self.show_missing_pool_notice(ctx, missing_pool);
        applied
    }

    /// 待機cycle用の複製へ抽選結果を載せる。候補が空なら複製時の現在値を残す。
    pub(crate) fn apply_random_patches_to(
        &mut self,
        instances: &mut [GridInstance],
        ctx: &GridSequencerContext<'_>,
    ) -> usize {
        let (assignments, missing_pool) =
            self.draw_patch_assignments(instances, ctx, false, PatchAssignmentScope::All);
        let mut applied = 0;
        for (instance, patch) in instances.iter_mut().zip(assignments) {
            if let Some(patch) = patch {
                instance.patch = Some(patch);
                applied += 1;
            }
        }
        self.show_missing_pool_notice(ctx, missing_pool);
        applied
    }

    fn show_missing_pool_notice(&mut self, ctx: &GridSequencerContext<'_>, missing: bool) {
        if !missing {
            self.patch_notice = None;
            return;
        }
        let reason = catalog_unavailable(ctx).unwrap_or(PatchUnavailable::NoRolePatches);
        self.patch_notice = Some(PatchNotice::new(reason, std::time::Instant::now()));
    }
}

struct PatchDrawBag {
    source: Vec<String>,
    remaining: Vec<String>,
}

impl PatchDrawBag {
    fn new(source: Vec<String>) -> Self {
        Self {
            source,
            remaining: Vec::new(),
        }
    }

    fn draw(&mut self) -> Option<String> {
        if self.remaining.is_empty() {
            self.remaining.clone_from(&self.source);
            self.remaining.shuffle(&mut rand::rng());
        }
        self.remaining.pop()
    }
}

#[derive(Clone, Copy)]
enum PatchAssignmentScope {
    All,
    DedicatedExceptChord,
}

impl PatchAssignmentScope {
    fn includes(self, purpose: GridPatchPurpose) -> bool {
        match self {
            Self::All => true,
            Self::DedicatedExceptChord => {
                !matches!(purpose, GridPatchPurpose::Note | GridPatchPurpose::Chord)
            }
        }
    }
}
