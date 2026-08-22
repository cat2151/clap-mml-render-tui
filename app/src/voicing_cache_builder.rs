//! `cmrt build-voicing-cache`: 全 patch の mono/poly 判定を一括で行い、
//! 結果を voicing_cache.json に保存する。
//!
//! keyboard 画面は起動時にこのキャッシュを読み込み、載っている patch では
//! probe を省略する。事前に一度これを実行しておけば、patch 切替時の判定待ちが
//! 発生しなくなる。

use anyhow::Result;

use cmrt_tui_core::patch_plugins::PatchPlugins;

use crate::{
    config::Config,
    history::{load_voicing_cache, save_voicing_cache},
    patches::collect_patch_pairs,
    realtime_play::RealtimePlayServerSupervisor,
};

/// 途中で中断されても成果が残るよう、この件数ごとに保存する。
const SAVE_INTERVAL: usize = 20;

pub fn run_build_voicing_cache(cfg: &Config, force: bool) -> Result<()> {
    let pairs = collect_patch_pairs(cfg)?;
    if pairs.is_empty() {
        println!(
            "patch が見つかりませんでした。config の patch ディレクトリ設定を確認してください。"
        );
        return Ok(());
    }

    // probe は CLAP note event の NOTE_END を数える方式なので、note dialect が MIDI
    // だけのプラグイン（Dexed / Vaporizer2）では「mono」と「測れなかった」を区別できない。
    // そういう音色は probe せず `VoicingPolicy` に任せる。Dexed は poly へ倒し
    // （`AssumePoly`）、Vaporizer2 は `.vvp` の `m_uPolyMode` を読む（`VvpHeader`）ので、
    // どちらもこのキャッシュを必要としない。
    let patch_plugins = PatchPlugins::from_config(cfg);
    let mut cache = load_voicing_cache();
    let probable = pairs
        .iter()
        .map(|(display, _)| display.as_str())
        .filter(|patch| patch_plugins.for_patch(patch).is_surge_xt())
        .collect::<Vec<_>>();
    let targets = probable
        .iter()
        .copied()
        .filter(|patch| force || cache.get(patch).is_none())
        .collect::<Vec<_>>();

    println!(
        "patch {} 件中 probe できるのは {} 件、うち {} 件を判定します（判定済みをスキップ）。",
        pairs.len(),
        probable.len(),
        targets.len()
    );
    if targets.is_empty() {
        return Ok(());
    }

    let supervisor = RealtimePlayServerSupervisor::new(cfg);
    let total = targets.len();
    let mut unsaved = 0usize;
    let mut failed = 0usize;

    for (index, patch) in targets.into_iter().enumerate() {
        print!("[{}/{}] {patch} -> ", index + 1, total);
        // 1 個の壊れた patch で全体が止まらないよう、失敗は表示して続行する。
        match supervisor.prepare_live_patch_with_voicing(0, Some(patch)) {
            Ok(Some(report)) => {
                println!("{}", voicing_label(report.decision));
                if cache.insert(patch, report.decision) {
                    unsaved += 1;
                }
            }
            Ok(None) => {
                println!("unavailable（play-server が probe に未対応です）");
                failed += 1;
            }
            Err(error) => {
                println!("error: {error}");
                failed += 1;
            }
        }
        if unsaved >= SAVE_INTERVAL {
            save_voicing_cache(&cache)?;
            unsaved = 0;
        }
    }

    save_voicing_cache(&cache)?;
    let _ = supervisor.stop();
    println!(
        "完了しました。判定済み {} 件 / 失敗 {} 件。",
        cache.entries.len(),
        failed
    );
    Ok(())
}

fn voicing_label(voicing: crate::realtime_play::PatchVoicing) -> &'static str {
    match voicing {
        crate::realtime_play::PatchVoicing::Mono => "mono",
        crate::realtime_play::PatchVoicing::Poly => "poly",
        crate::realtime_play::PatchVoicing::Unknown => "unknown",
    }
}
