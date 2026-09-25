//! コマンドラインの受け取り。**何をするかを決めるだけ**で、実行はしない。
//!
//! 実行は `main.rs` の `run()`。ここが返す [`CliInvocation`] が
//! 「どのモードか」と「全モード共通の指定」の 2 つを持つ。

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_mml_render_tui::{
    bass_voicing_inspect::BassVoicingInspectRequest,
    live_chord_check::LiveChordCheckRequest,
    live_line_check::{LiveLineCheckRequest, ResidualRequest},
    render_mml::RenderMmlRequest,
    server,
};
use std::path::PathBuf;

use crate::cli_output;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CliAction {
    Help(String),
    Version(String),
    Tui,
    CliMml(String),
    Server(u16),
    Shutdown(u16),
    Update,
    Check,
    ScanLoops,
    BuildVoicingCache { force: bool },
    BuildPatchCatalogCache,
    PatchRoles { config: Option<PathBuf> },
    RenderMml(RenderMmlRequest),
    LiveChordCheck(LiveChordCheckRequest),
    LiveLineCheck(LiveLineCheckRequest),
    InspectBassVoicing(BassVoicingInspectRequest),
}

#[derive(Debug, Parser)]
#[command(
    name = "cmrt",
    about = "CLAP MML Render TUI",
    args_conflicts_with_subcommands = true,
    disable_help_subcommand = true
)]
struct Cli {
    #[arg(
        long,
        num_args = 0..=1,
        value_name = "PORT",
        conflicts_with = "shutdown",
        help = "サーバーモードで起動する"
    )]
    server: Option<Option<u16>>,

    #[arg(
        long,
        num_args = 0..=1,
        value_name = "PORT",
        conflicts_with = "server",
        help = "起動中のサーバーを停止する"
    )]
    shutdown: Option<Option<u16>>,

    /// play server の実体を明示指定する。
    ///
    /// `global = true` なのは、サブコマンド（`render-mml` など）からも play server を
    /// 起こすため。`args_conflicts_with_subcommands` の対象からも外れる。
    #[arg(
        long = "play-server",
        global = true,
        value_name = "PATH",
        help = "play server の実体（フルパス）を指定する"
    )]
    play_server: Option<PathBuf>,

    #[arg(long = "mml", hide = true, num_args = 0..=1, value_name = "MML")]
    deprecated_mml: Option<Option<String>>,

    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(value_name = "MML", help = "CLI モードで再生する MML（テスト用）")]
    mml: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// アップデートを実行
    Update,
    /// ビルド時コミットと remote main を比較
    Check,
    /// 全 patch の mono/poly を判定してキャッシュを作成する
    BuildVoicingCache {
        /// 判定済みの patch も含めて全件を再判定する
        #[arg(long)]
        force: bool,
    },
    /// TUIが読むpatch catalog cacheを現在のconfigから再構築する
    BuildPatchCatalogCache,
    /// loop_dirs を走査して WAV ループキャッシュを再構築する
    ScanLoops,
    /// MML 1 本をオフラインでレンダリングして出音を数字で出す（画面を起動しない動作確認）
    RenderMml {
        /// 既定の置き場ではなく、この config.toml を読む
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        /// 鳴らす音色の display 文字列。複数指定すると 1 プロセスで順に鳴らして比べる
        #[arg(long = "patch", value_name = "DISPLAY")]
        patches: Vec<String>,
        /// 指定プラグインへ routing される共有カタログ上の全音色を鳴らす
        #[arg(long, value_name = "NAME", conflicts_with = "patches")]
        plugin: Option<String>,
        /// WAV の書き出し先（省略時は環境変数 CMRT_TEST_WAV_OUT_DIR。どちらも無ければ書かない）
        #[arg(long, value_name = "DIR")]
        out_dir: Option<PathBuf>,
        /// 和音が本当に和音で鳴るかを、単音のレンダリングと突き合わせて判定する
        #[arg(long)]
        poly_check: bool,
        /// 0件・誤 routing・load/render error・無音を失敗終了にする
        #[arg(long)]
        verify: bool,
        /// レンダリングする MML（省略時は 1 音だけ鳴らす既定 MML）
        #[arg(value_name = "MML")]
        mml: Option<String>,
    },
    /// Chord Chart の和音送りを realtime server で再現し、出音を測る
    LiveChordCheck {
        /// 既定の置き場ではなく、この config.toml を読む
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        /// Chord Chart で使う音色の display 文字列
        #[arg(long, value_name = "DISPLAY")]
        patch: String,
        /// chord2mml へ渡す Key トークン
        #[arg(long, default_value = "Key=C", value_name = "TOKEN")]
        key: String,
        /// 次の和音へ送るまでの待機時間
        #[arg(long, default_value_t = 1000, value_name = "MS")]
        step_ms: u64,
        /// server が取った live mix の WAV 保存先
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// 無音の chord または短すぎる capture を失敗終了にする
        #[arg(long)]
        verify: bool,
        /// Chord Chart と同じ degrees 文字列
        #[arg(value_name = "DEGREES", default_value = "I-V-VIm-IV")]
        degrees: String,
    },
    /// MML の行を LIVE で順に鳴らし、device へ出た波形のクリック・途切れ・頭を測る
    LiveLineCheck {
        /// 既定の置き場ではなく、この config.toml を読む
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        /// 次の行へ送るまでの間隔
        #[arg(long, default_value_t = 1000, value_name = "MS")]
        step_ms: u64,
        /// server が device へ出した波形の WAV 保存先
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// 同じ行を offline render した対照も解析する
        #[arg(long)]
        offline: bool,
        /// 2 行目以降を送る直前に、鳴っている前の行をこの長さで fadeout する（EFFECT CHAIN の試聴と同じ）
        #[arg(long, value_name = "MS", value_parser = clap::value_parser!(u32).range(1..=10_000))]
        fade_previous_ms: Option<u32>,
        /// 1 行目だけを鳴らした録音。2 行目の送信の後に残る 1 行目の音をこれと比べる
        #[arg(long, value_name = "PATH", requires = "residual_notch_note")]
        residual_reference: Option<PathBuf>,
        /// 前の行の残りを測るときに除く、2 行目の音の MIDI note number（12 平均律 A4=440 Hz）
        #[arg(long, value_name = "NOTE", requires = "residual_reference", value_parser = clap::value_parser!(u8).range(0..=127))]
        residual_notch_note: Option<u8>,
        /// 先頭 JSON 込みの MML の行。この順に送る
        #[arg(value_name = "LINE", required = true, num_args = 1..)]
        lines: Vec<String>,
    },
    /// 複数のコード進行を連結してauto voiceし、Bass note numberを表示する
    InspectBassVoicing {
        /// chord2mmlへ渡すKeyトークン
        #[arg(long, default_value = "Key=C", value_name = "TOKEN")]
        key: String,
        /// Chord Chartのsection順に並べたdegrees文字列（全引数を1進行としてvoiceする）
        #[arg(value_name = "DEGREES", required = true, num_args = 1..)]
        progressions: Vec<String>,
    },
    /// grid sequencer の各行に patch の候補が出るかを調べる（画面を起動しない動作確認）
    PatchRoles {
        /// 既定の置き場ではなく、この config.toml を読む（実ユーザーの設定を書き換えずに
        /// `[plugins.*]` を試すため）
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
}

fn cli_command() -> clap::Command {
    Cli::command()
        .version(cli_output::version_text())
        .after_help(format!(
            "サーバーモードでは HTTP POST でMMLを受け取りWAVデータを返します。\n  例: curl -X POST http://127.0.0.1:{}/ --data 'cde'",
            server::DEFAULT_PORT
        ))
}

/// 1 回の起動で決まること。`action` に加えて、全モード共通の指定を持つ。
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CliInvocation {
    pub(crate) action: CliAction,
    /// `--play-server <PATH>`。**探索より強い**（ADR 0017）。
    pub(crate) play_server: Option<PathBuf>,
}

/// `action` だけが要るとき用（テストの読みやすさのため）。
#[cfg(test)]
fn parse_cli_from<I, T>(args: I) -> Result<CliAction>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    parse_cli_invocation_from(args).map(|invocation| invocation.action)
}

pub(crate) fn parse_cli_invocation_from<I, T>(args: I) -> Result<CliInvocation>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = match cli_command().try_get_matches_from_mut(args) {
        Ok(matches) => {
            Cli::from_arg_matches(&matches).map_err(|err| anyhow::anyhow!(err.to_string()))?
        }
        Err(err) if err.kind() == clap::error::ErrorKind::DisplayHelp => {
            return Ok(CliInvocation {
                action: CliAction::Help(err.to_string()),
                play_server: None,
            });
        }
        Err(err) if err.kind() == clap::error::ErrorKind::DisplayVersion => {
            return Ok(CliInvocation {
                action: CliAction::Version(err.to_string()),
                play_server: None,
            });
        }
        Err(err) => return Err(anyhow::anyhow!(err.to_string())),
    };

    if cli.deprecated_mml.is_some() {
        anyhow::bail!(
            "`--mml` オプションは廃止されました。`cmrt <mml>` の形式で指定してください。\n例: cmrt cde"
        );
    }

    // `--play-server` はどのモードでも同じ意味なので、action の分岐とは別に持つ。
    let play_server = cli.play_server.clone();
    let wrap = |action| {
        Ok(CliInvocation {
            action,
            play_server: play_server.clone(),
        })
    };

    if let Some(port) = cli.shutdown {
        let port = port.unwrap_or(server::DEFAULT_PORT);
        return wrap(CliAction::Shutdown(port));
    }

    if let Some(port) = cli.server {
        let port = port.unwrap_or(server::DEFAULT_PORT);
        return wrap(CliAction::Server(port));
    }

    if matches!(cli.command, Some(Commands::Update)) {
        return wrap(CliAction::Update);
    }

    if matches!(cli.command, Some(Commands::Check)) {
        return wrap(CliAction::Check);
    }

    if matches!(cli.command, Some(Commands::ScanLoops)) {
        return wrap(CliAction::ScanLoops);
    }

    if let Some(Commands::PatchRoles { config }) = cli.command {
        return wrap(CliAction::PatchRoles { config });
    }

    if let Some(Commands::RenderMml {
        config,
        patches,
        plugin,
        out_dir,
        poly_check,
        verify,
        mml,
    }) = cli.command
    {
        return wrap(CliAction::RenderMml(RenderMmlRequest {
            config,
            patches,
            plugin,
            mml,
            out_dir,
            poly_check,
            verify,
        }));
    }

    if let Some(Commands::LiveChordCheck {
        config,
        patch,
        key,
        step_ms,
        out,
        verify,
        degrees,
    }) = cli.command
    {
        return wrap(CliAction::LiveChordCheck(LiveChordCheckRequest {
            config,
            patch,
            key,
            degrees,
            step_ms,
            out,
            verify,
        }));
    }

    if let Some(Commands::LiveLineCheck {
        config,
        step_ms,
        out,
        offline,
        fade_previous_ms,
        residual_reference,
        residual_notch_note,
        lines,
    }) = cli.command
    {
        return wrap(CliAction::LiveLineCheck(LiveLineCheckRequest {
            config,
            lines,
            step_ms,
            out,
            offline,
            fade_previous_ms,
            residual: residual_reference
                .zip(residual_notch_note)
                .map(|(reference, notch_note)| ResidualRequest {
                    reference,
                    notch_note,
                }),
        }));
    }

    if let Some(Commands::InspectBassVoicing { key, progressions }) = cli.command {
        return wrap(CliAction::InspectBassVoicing(BassVoicingInspectRequest {
            key,
            progressions,
        }));
    }

    if let Some(Commands::BuildVoicingCache { force }) = cli.command {
        return wrap(CliAction::BuildVoicingCache { force });
    }

    if matches!(cli.command, Some(Commands::BuildPatchCatalogCache)) {
        return wrap(CliAction::BuildPatchCatalogCache);
    }

    if let Some(mml) = cli.mml {
        return wrap(CliAction::CliMml(mml));
    }

    wrap(CliAction::Tui)
}

/// `--play-server <PATH>` を実体の指定へ変える。
///
/// 存在しないパスを黙って探索へ落とさない。打った指定が静かに無視されるのは、
/// ADR 0017 が潰した事故（環境が黙って実体を決める）と同じ手触りになる。
pub(crate) fn play_server_launch(path: PathBuf) -> Result<cmrt_runtime::PlayServerLaunch> {
    if !path.is_file() {
        anyhow::bail!(
            "--play-server で指定された実体がありません: {}",
            path.display()
        );
    }
    Ok(cmrt_runtime::PlayServerLaunch::Executable(path))
}

#[cfg(test)]
mod tests;
