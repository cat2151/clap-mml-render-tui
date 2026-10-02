//! サブコマンドの定義（clap derive）。解釈した値を `CliAction` へ詰めるのは親モジュール。

use clap::Subcommand;
use std::path::PathBuf;

use super::keyboard_args::KeyboardArgs;

#[derive(Debug, Subcommand)]
pub(super) enum Commands {
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
    /// daily DAW の全セルを画面と同じ MML で render し直し、今ある cache WAV と比べる
    InspectDawCache {
        /// 新しく render した WAV の書き出し先（省略時は書かない）
        #[arg(long, value_name = "DIR")]
        out_dir: Option<PathBuf>,
        /// 「早く鳴り止む」と判定したセルの cache WAV を消す（次に DAW を開くと render し直される）
        #[arg(long)]
        delete_broken: bool,
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
    /// Guitar Articulation 画面と同じ変換で、音ごとの奏法とイベント列を出す（画面を起動しない分析）
    GuitarArticulationEvents {
        /// ログの最後の演奏（画面で鳴らした MML とルール表）から作り直す
        #[arg(long, conflicts_with_all = ["rules", "mml", "compare_previous"])]
        last_played: bool,
        /// ログの最後の演奏と、同じ MML でルールが違う直前の演奏を、音ごとに並べて比べる
        #[arg(long, conflicts_with_all = ["rules", "mml"])]
        compare_previous: bool,
        /// ルール表の JSON。例: {"columns":{"3":["hammer_pull"]},"rows":["economy_picking"]}
        #[arg(long, value_name = "JSON")]
        rules: Option<String>,
        /// 変換する MML
        #[arg(value_name = "MML", required_unless_present_any = ["last_played", "compare_previous"])]
        mml: Option<String>,
    },
    /// grid sequencer の各行に patch の候補が出るかを調べる（画面を起動しない動作確認）
    PatchRoles {
        /// 既定の置き場ではなく、この config.toml を読む（実ユーザーの設定を書き換えずに
        /// `[plugins.*]` を試すため）
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
    /// keyboard 画面で起動する。書いた項目で keyboard の状態を差し替える
    Kb(KeyboardArgs),
    /// Dexed の program のうち、送る SysEx が同じ（＝同じ音色）ものの重複を数える
    DexedDuplicates {
        /// 既定の置き場ではなく、この config.toml を読む
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        /// patch 選択画面と同じ絞り込み条件（空白区切りの正規表現を AND）。省略時は Dexed の全 program
        #[arg(value_name = "CONDITION", num_args = 0..)]
        condition: Vec<String>,
    },
}
