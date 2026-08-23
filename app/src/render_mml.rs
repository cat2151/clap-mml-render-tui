//! MML 1 本をオフラインでレンダリングして、出音を**数字で** print する診断コマンド。
//!
//! # なぜ要るか
//! オフライン経路（notepad の WAV 書き出し / DAW のセルキャッシュ）が本当に鳴るかは、
//! これまで画面を起動して耳で確かめるしかなかった。混在カタログではそこが
//! 「どのプラグインへ渡ったか」の唯一の確認手段になるので、**画面を起動せずに
//! 通せる口**を用意する（`docs/adr/0011-verification-and-baselines.md`）。
//!
//! `cmrt patch-roles` が「一覧に出るか」を数えるのに対し、こちらは
//! **「出た音色が実際に音になるか」**を見る。
//!
//! # 判定に使える数字
//! - `rms` / `peak` — 無音でないこと
//! - `digest` — 音色を替えたら**出音が変わる**こと（同じなら差し替わっていない）
//! - `--poly-check` — 和音が**本当に和音で鳴る**こと（単音のレンダリングと突き合わせる）
//!
//! WAV は `--out-dir`（無ければ環境変数 `CMRT_TEST_WAV_OUT_DIR`）が指定されたときだけ書く。
//! 耳で確かめたいぶんはそこへ溜めて、あとでまとめて聴く。

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use cmrt_offline_render::{OfflineRenderer, PluginEntries};
use cmrt_runtime::{Config, OfflineRenderBackend};
use cmrt_tui_core::patch_plugins::PatchPlugins;

mod analysis;
mod selection;

pub(crate) use analysis::RenderStats;

/// `cmrt render-mml` の引数。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RenderMmlRequest {
    /// 既定の置き場ではなくこの config.toml を読む。実ユーザーの設定を書き換えないため。
    pub config: Option<PathBuf>,
    /// 鳴らす音色の display 文字列。複数指定すると 1 プロセスで順に鳴らして比べる。
    pub patches: Vec<String>,
    /// 共有カタログから、このプラグインへ routing される全音色を選ぶ。
    pub plugin: Option<String>,
    /// レンダリングする MML。省略時は [`DEFAULT_MML`]。
    pub mml: Option<String>,
    /// WAV の書き出し先。省略時は環境変数 [`WAV_OUT_DIR_ENV`]。どちらも無ければ書かない。
    pub out_dir: Option<PathBuf>,
    /// 和音が本当に和音で鳴るかを、単音のレンダリングと突き合わせて判定する。
    pub poly_check: bool,
    /// 無音・0件・routing 不一致を終了コードで失敗にする。
    pub verify: bool,
}

/// 音色だけを見たいときの既定 MML。1 音を全音符 1 つぶん。
pub const DEFAULT_MML: &str = "t120v11'c1'";

/// verification 向け既定。低音から高音まで順に鳴らし、drum の狭い key range も拾う。
pub const VERIFY_DEFAULT_MML: &str = "t180v11o2l16cdefgabo4cdefgabo6cdefgab";

/// `--out-dir` を省略したときに見る環境変数。play-server 側の番人テストと同じ名前。
pub const WAV_OUT_DIR_ENV: &str = "CMRT_TEST_WAV_OUT_DIR";

/// poly-check で鳴らす和音。**chord2mml を通さず生 MML で書く。**
///
/// 単音側と和音側で音長・音量・オクターブが 1 つでもずれると、mono の音色でも
/// 「単音と一致しない」になって判定が壊れる。同じ書式で並べて、ずれようがなくする。
const POLY_CHECK_CHORD_MML: &str = "t120v11'c1eg'";

/// poly-check で 1 つずつ鳴らす、上の和音の構成音。
const POLY_CHECK_NOTES: [(&str, &str); 3] = [
    ("c", "t120v11'c1'"),
    ("e", "t120v11'e1'"),
    ("g", "t120v11'g1'"),
];

/// これ以上なら和音が鳴っているとみなす音量比。3 音が非干渉に重なれば `sqrt(3)` ≒ 1.73。
const POLY_ENERGY_GAIN: f64 = 1.25;

/// これ以下なら 1 音しか鳴っていないとみなす音量比。
const MONO_ENERGY_GAIN: f64 = 1.10;

/// 同じ MML を 2 回鳴らしたときに許す RMS のぶれ。超えたら判定しない。
const MAX_RMS_JITTER: f64 = 0.10;

pub fn run(cfg: &Config, entries: &PluginEntries, request: &RenderMmlRequest) -> Result<()> {
    let mml = request.mml.as_deref().unwrap_or(if request.verify {
        VERIFY_DEFAULT_MML
    } else {
        DEFAULT_MML
    });
    let out_dir = resolve_out_dir(request)?;
    let catalog = PatchPlugins::from_catalog(cmrt_runtime::catalog_plugins(cfg));
    let patches = selection::requested_patches(cfg, &catalog, request)?;
    let renderer = OfflineRenderer::new(std::sync::Arc::new(cfg.clone()), entries.clone());

    println!("[render-mml]");
    println!("  config        : {}", describe_config(request));
    println!("  backend       : {}", backend_label(cfg));
    println!("  sample_rate   : {}", cfg.sample_rate);
    println!(
        "  out_dir       : {}",
        out_dir
            .as_ref()
            .map(|dir| dir.display().to_string())
            .unwrap_or_else(|| "(WAV を書かない)".to_string())
    );

    if request.poly_check {
        return run_poly_check(cfg, &renderer, &catalog, &patches, out_dir.as_deref());
    }
    run_renders(
        cfg,
        &renderer,
        &catalog,
        request,
        &patches,
        mml,
        out_dir.as_deref(),
    )
}

fn run_renders(
    cfg: &Config,
    renderer: &OfflineRenderer,
    catalog: &PatchPlugins,
    request: &RenderMmlRequest,
    patches: &[Option<String>],
    mml: &str,
    out_dir: Option<&Path>,
) -> Result<()> {
    println!("  mml           : {mml}");
    println!();

    // **サンプル列は溜めない。** 1 本 4 秒のステレオで 1.5MB あり、460 音色を
    // 一度に流すと 700MB になる。まとめに要るのは digest と無音判定だけ。
    let mut renders = Vec::new();
    let mut silent = Vec::new();
    for patch in patches {
        let stat = render_one(cfg, renderer, catalog, patch.as_deref(), mml, out_dir)?;
        let name = patch.clone().unwrap_or_else(|| "(無指定)".to_string());
        renders.push((name.clone(), stat.digest));
        if stat.is_silent() {
            silent.push(name);
        }
    }

    println!();
    println!("[まとめ]");
    println!("  レンダリング数 : {}", renders.len());
    println!("  load/render error: 0 件");
    println!("  無音           : {} 件", silent.len());
    for patch in &silent {
        println!("    無音: {patch}");
    }
    println!(
        "  異なる出音     : {} / {}",
        analysis::distinct_digests(
            &renders
                .iter()
                .map(|(_, digest)| *digest)
                .collect::<Vec<_>>()
        ),
        renders.len()
    );
    print_duplicate_digests(&renders);
    if request.verify && !silent.is_empty() {
        anyhow::bail!("verify 失敗: 無音の音色が {} 件あります", silent.len());
    }
    Ok(())
}

/// 和音が本当に和音で鳴っているかを、単音のレンダリングとの**音量差**で判定する。
///
/// # なぜ「波形の一致」ではなく音量で見るか
/// 最初は「mono なら和音のレンダリングが単音のどれかと一致するはず」で判定したが、
/// **実測で外れた**。mono の音色（`SY Analog Taste 001.vvp`）へ和音を送ると、
/// 鳴るのは 1 音だけなのに波形は単音と一致しない（`diff_ratio` は最寄りでも 0.42）。
/// mono のノート優先やエンベロープの再トリガが波形を変えるため。
/// さらに poly の音色には**同じ MML でも毎回違う波形を出すもの**がある
/// （`AT Ambience 1.vvp` はグラニュラで、2 回鳴らすと digest が変わる）。
///
/// 音量（RMS）はどちらにも強い。実測はこうなった:
///
/// | 音色 | 和音の RMS | 単音の RMS 平均 | 比 |
/// |---|---|---|---|
/// | `SY Analog Taste 001.vvp`（Mono） | 0.084866 | 0.084869 | **1.00** |
/// | `AT Ambience 1.vvp`（Poly16） | 0.080775 | 0.049393 | **1.64** |
///
/// 3 音が非干渉に重なれば `sqrt(3)` ≒ 1.73、1 音しか鳴らなければ 1.00。
/// 実測はその両端に乗る。[`POLY_ENERGY_GAIN`] / [`MONO_ENERGY_GAIN`] の間は
/// **どちらとも言わない**（黙って poly 側へ倒すと mono を和音行へ通してしまう）。
fn run_poly_check(
    cfg: &Config,
    renderer: &OfflineRenderer,
    catalog: &PatchPlugins,
    patches: &[Option<String>],
    out_dir: Option<&Path>,
) -> Result<()> {
    let chord_mml = POLY_CHECK_CHORD_MML;
    println!("  chord         : {chord_mml}");
    println!();

    for patch in patches {
        let patch = patch.as_deref();
        let chord = render_one(cfg, renderer, catalog, patch, chord_mml, out_dir)?;
        // 同じ MML を 2 回。RMS がぶれる音色かどうかを、判定と同じ土俵で測っておく。
        let again = render_one(cfg, renderer, catalog, patch, chord_mml, None)?;

        let mut note_rms = Vec::new();
        for (name, note_mml) in POLY_CHECK_NOTES {
            let note = render_one(cfg, renderer, catalog, patch, note_mml, out_dir)?;
            println!(
                "    単音 {name}: rms={:.6} 和音との波形差 diff_ratio={:.4}",
                note.rms,
                analysis::diff_ratio(&chord, &note)
            );
            note_rms.push(note.rms);
        }

        let measure = PolyCheck::of(chord.rms, again.rms, &note_rms);
        println!(
            "  poly-check patch='{}' chord_rms={:.6} note_rms_mean={:.6} energy_gain={:.3} rms_jitter={:.4} same_waveform_twice={} verdict={}",
            patch.unwrap_or("(無指定)"),
            measure.chord_rms,
            measure.note_rms_mean,
            measure.energy_gain,
            measure.rms_jitter,
            if again.digest == chord.digest { "yes" } else { "no" },
            measure.verdict(),
        );
        println!();
    }
    Ok(())
}

/// poly-check 1 音色ぶんの測定値。
#[derive(Clone, Copy, Debug)]
pub(crate) struct PolyCheck {
    pub(crate) chord_rms: f64,
    pub(crate) note_rms_mean: f64,
    /// 和音の音量が単音の何倍か。1.00 なら 1 音しか鳴っていない。
    pub(crate) energy_gain: f64,
    /// 同じ MML を 2 回鳴らしたときの RMS のぶれ。大きいと [`energy_gain`] も信用できない。
    ///
    /// [`energy_gain`]: PolyCheck::energy_gain
    pub(crate) rms_jitter: f64,
}

impl PolyCheck {
    pub(crate) fn of(chord_rms: f64, chord_rms_again: f64, note_rms: &[f64]) -> Self {
        let note_rms_mean = if note_rms.is_empty() {
            0.0
        } else {
            note_rms.iter().sum::<f64>() / note_rms.len() as f64
        };
        Self {
            chord_rms,
            note_rms_mean,
            energy_gain: if note_rms_mean > 0.0 {
                chord_rms / note_rms_mean
            } else {
                f64::NAN
            },
            rms_jitter: if chord_rms > 0.0 {
                (chord_rms - chord_rms_again).abs() / chord_rms
            } else {
                0.0
            },
        }
    }

    /// poly-check の判定文。
    pub(crate) fn verdict(&self) -> &'static str {
        if self.chord_rms <= 0.0 || self.note_rms_mean <= 0.0 {
            return "unknown(無音なので比べられない)";
        }
        if self.rms_jitter > MAX_RMS_JITTER {
            // 同じ MML で音量が動く音色は、和音と単音の比も同じだけ動く。
            return "unknown(同じMMLで音量が変わるので比べられない)";
        }
        if self.energy_gain >= POLY_ENERGY_GAIN {
            "poly(和音が単音より大きい)"
        } else if self.energy_gain <= MONO_ENERGY_GAIN {
            "mono(和音でも音量が単音のまま)"
        } else {
            "unclear(どちらとも言えない)"
        }
    }
}

/// 音色を 1 つ当てて MML を 1 本レンダリングし、1 行 print する。
///
/// `mml` は音色を埋める前の裸の MML。先頭 JSON はここで足す。**WAV のファイル名も
/// これで作る**ので、埋めたあとの文字列を渡すと音色名が名前に 2 回入る。
fn render_one(
    cfg: &Config,
    renderer: &OfflineRenderer,
    catalog: &PatchPlugins,
    patch: Option<&str>,
    mml: &str,
    out_dir: Option<&Path>,
) -> Result<RenderStats> {
    let plugin = plugin_name_for(catalog, patch);
    let line = mml_with_patch(patch, mml);
    let started = std::time::Instant::now();
    let output = renderer
        .render_phrase(&line, None)
        .with_context(|| format!("オフラインレンダリングに失敗しました: mml={line}"))?;
    let elapsed_ms = started.elapsed().as_millis();
    let stats = RenderStats::of(&output.samples, cfg.sample_rate as u32, &output.patch_name);

    let wav = match out_dir {
        Some(dir) => Some(write_wav(
            dir,
            patch,
            mml,
            &output.samples,
            cfg.sample_rate as u32,
        )?),
        None => None,
    };

    println!(
        "  render patch='{}' plugin={plugin} frames={} duration_ms={} peak={:.4} rms={:.6} silent={} digest={:016x} elapsed_ms={elapsed_ms} patch_name='{}' wav={}",
        patch.unwrap_or("(無指定)"),
        stats.frames,
        stats.duration_ms,
        stats.peak,
        stats.rms,
        if stats.is_silent() { "yes" } else { "no" },
        stats.digest,
        stats.patch_name,
        wav.as_ref()
            .map(|path| format!("'{}'", path.display()))
            .unwrap_or_else(|| "-".to_string()),
    );
    Ok(stats)
}

/// この音色を鳴らすプラグインの名前。引き分けは PATCH 欄の wheel と同じ述語を通す。
///
/// **音色を無指定にした MML は必ず既定プラグイン（先頭）**
/// （`docs/adr/0004-default-plugin-owns-unspecified-patches.md`）。patch 文字列の形で
/// 引くと、空文字列が「cartridge でも .vvp でもない」と判定されて別のプラグインへ飛ぶ。
pub(crate) fn plugin_name_for(catalog: &PatchPlugins, patch: Option<&str>) -> String {
    match patch {
        None => catalog.plugins().first().map(|plugin| plugin.name.as_str()),
        Some(patch) => Some(catalog.for_patch(patch).name.as_str()),
    }
    .unwrap_or("(カタログが空)")
    .to_string()
}

fn print_duplicate_digests(renders: &[(String, u64)]) {
    let mut by_digest = std::collections::BTreeMap::<u64, Vec<&str>>::new();
    for (patch, digest) in renders {
        by_digest.entry(*digest).or_default().push(patch);
    }
    let duplicates = by_digest
        .into_iter()
        .filter(|(_, patches)| patches.len() > 1)
        .collect::<Vec<_>>();
    println!("  重複 digest    : {} 組", duplicates.len());
    for (digest, patches) in duplicates {
        println!("    {digest:016x}: {}", patches.join(" | "));
    }
}

/// MML 先頭 JSON へ音色を埋める。キーは `"Surge XT patch"` のまま
/// （`docs/adr/0001-patch-string-decides-the-plugin.md`。どのプラグインで鳴るかは
/// **値の形**で決まるので、キーの綴りは 3 種別に増えても変えない）。
pub(crate) fn mml_with_patch(patch: Option<&str>, mml: &str) -> String {
    let Some(patch) = patch else {
        return mml.to_string();
    };
    let json = serde_json::json!({ "Surge XT patch": patch });
    format!("{json}{mml}")
}

fn resolve_out_dir(request: &RenderMmlRequest) -> Result<Option<PathBuf>> {
    let dir = match &request.out_dir {
        Some(dir) => Some(dir.clone()),
        None => std::env::var_os(WAV_OUT_DIR_ENV).map(PathBuf::from),
    };
    if let Some(dir) = &dir {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("WAV の書き出し先を作れませんでした: {}", dir.display()))?;
    }
    Ok(dir)
}

fn describe_config(request: &RenderMmlRequest) -> String {
    match &request.config {
        Some(path) => path.display().to_string(),
        None => "(既定の置き場)".to_string(),
    }
}

fn backend_label(cfg: &Config) -> &'static str {
    match cfg.offline_render_backend {
        OfflineRenderBackend::InProcess => "in_process(このプロセスで CLAP をホストする)",
        OfflineRenderBackend::RenderServer => "render_server(別プロセスの render server へ投げる)",
    }
}

fn write_wav(
    dir: &Path,
    patch: Option<&str>,
    mml: &str,
    samples: &[f32],
    sample_rate: u32,
) -> Result<PathBuf> {
    let path = dir.join(format!(
        "{}__{}.wav",
        analysis::file_stem_for(patch.unwrap_or("no-patch")),
        analysis::file_stem_for(mml)
    ));
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&path, spec)
        .with_context(|| format!("WAV を作れませんでした: {}", path.display()))?;
    for &sample in samples {
        writer
            .write_sample(sample)
            .context("WAV のサンプル書き込みに失敗しました")?;
    }
    writer
        .finalize()
        .context("WAV の finalize に失敗しました")?;
    Ok(path)
}

#[cfg(test)]
mod tests;
