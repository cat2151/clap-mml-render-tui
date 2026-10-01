use anyhow::Result;
use cmrt_realtime_play::{PatchVoicing, RealtimePlayServerSupervisor, VoicingReport};

use super::KEYBOARD_INSTANCE;

/// worker が play server へ送る、音色の準備の 1 手。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PrepareStep {
    /// 音色を読み、voicing を判定する。chain は載せられない。
    ProbeVoicing,
    /// 音色と effect chain を一緒に載せる（chain が空なら chain を外す）。
    PrepareWithChain,
}

/// 何をきっかけに準備を送るか。
#[derive(Clone, Copy, Debug)]
enum PrepareTrigger {
    /// 接続の準備・音色の差し替え。
    Patch { known_voicing: Option<PatchVoicing> },
    /// chain だけの差し替え。`patch_prepared` は、この接続で音色の準備が通ったか。
    EffectChain { patch_prepared: bool },
}

/// chain 無しの準備を同じ音色で送るとそれだけで chain が外れるので、
/// chain を載せない手（probe）の後には、chain が空でない限り chain 付きの準備を必ず続ける。
fn prepare_steps(trigger: PrepareTrigger, effect_chain: &str) -> Vec<PrepareStep> {
    match trigger {
        PrepareTrigger::Patch {
            known_voicing: Some(_),
        } => vec![PrepareStep::PrepareWithChain],
        PrepareTrigger::Patch {
            known_voicing: None,
        } if effect_chain.is_empty() => vec![PrepareStep::ProbeVoicing],
        PrepareTrigger::Patch {
            known_voicing: None,
        } => vec![PrepareStep::ProbeVoicing, PrepareStep::PrepareWithChain],
        // 音色がまだ載っていない instance へ chain だけ送っても掛からない。
        // chain は worker が覚えていて、次の音色の準備に同梱される。
        PrepareTrigger::EffectChain {
            patch_prepared: false,
        } => Vec::new(),
        PrepareTrigger::EffectChain {
            patch_prepared: true,
        } => vec![PrepareStep::PrepareWithChain],
    }
}

/// worker が覚えている、instance 0 に載っている（はずの）音色と chain。
#[derive(Debug, Default)]
pub(super) struct LivePatchState {
    patch: Option<String>,
    effect_chain: String,
    /// この接続で音色の準備が通ったか。
    prepared: bool,
}

/// 1 つのコマンドで送る準備。
#[derive(Debug, PartialEq, Eq)]
pub(super) struct PreparePlan {
    pub(super) patch: Option<String>,
    pub(super) effect_chain: String,
    pub(super) steps: Vec<PrepareStep>,
}

impl LivePatchState {
    /// 音色の準備。`effect_chain` が `Some` なら覚えている chain を差し替えてから送る。
    pub(super) fn plan_patch(
        &mut self,
        patch: Option<String>,
        known_voicing: Option<PatchVoicing>,
        effect_chain: Option<String>,
    ) -> PreparePlan {
        self.patch = patch;
        if let Some(chain) = effect_chain {
            self.effect_chain = chain;
        }
        self.plan(PrepareTrigger::Patch { known_voicing })
    }

    /// chain だけの差し替え。音色は覚えている今のものを送り直す。
    pub(super) fn plan_effect_chain(&mut self, effect_chain: String) -> PreparePlan {
        self.effect_chain = effect_chain;
        self.plan(PrepareTrigger::EffectChain {
            patch_prepared: self.prepared,
        })
    }

    /// 音色の準備の結果。失敗した instance に何が載っているかは分からないので、
    /// 次の音色の準備が通るまで chain だけの送信を控える。
    pub(super) fn finish_patch(&mut self, succeeded: bool) {
        self.prepared = succeeded;
    }

    fn plan(&self, trigger: PrepareTrigger) -> PreparePlan {
        PreparePlan {
            patch: self.patch.clone(),
            effect_chain: self.effect_chain.clone(),
            steps: prepare_steps(trigger, &self.effect_chain),
        }
    }
}

/// `plan` の手を順に送る。voicing は probe したときだけ返る。
pub(super) fn run_plan(
    supervisor: &RealtimePlayServerSupervisor,
    plan: &PreparePlan,
) -> Result<Option<VoicingReport>> {
    let patch = plan.patch.as_deref();
    let mut report = None;
    for step in &plan.steps {
        match step {
            PrepareStep::ProbeVoicing => {
                report = supervisor.prepare_live_patch_with_voicing(KEYBOARD_INSTANCE, patch)?;
            }
            PrepareStep::PrepareWithChain => supervisor.prepare_live_patch_with_effect_chain(
                KEYBOARD_INSTANCE,
                patch,
                &plan.effect_chain,
            )?,
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests;
