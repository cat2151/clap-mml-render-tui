//! 実サーバーへ繋ぐテストの共通入口（テスト専用）。
//!
//! `live_cache` と `live_gain` の両方が「起動済みの play server へ本物のコマンドを送る、
//! **通常は skip される**テスト」を持つ。ポートとキャッシュ WAV のパスはマシン固有なので
//! **コードへ書かず環境変数で受け取る**。その受け取り方をここ 1 か所に集めてある。
//!
//! ```text
//! CMRT_REALTIME_PLAY_SERVER_PORT=8712 CMRT_LIVE_INSTANCE_COUNT=2 \
//!   ../clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe > server.log 2>&1 &
//! CMRT_LIVE_CACHE_TEST_PORT=8712 CMRT_LIVE_CACHE_TEST_WAV=<絶対パス> \
//!   cargo test -p cmrt-daw --lib live_ -- --test-threads=1
//! ```
//!
//! `--test-threads=1` が要る。`FastMidiClient` は 1 プロセス 1 接続なので、
//! 2 つの実サーバーテストが並列に走ると 2 本目の `connect()` が `AlreadyConnected` になる。

use std::path::PathBuf;

use cmrt_realtime_play::RealtimePlayServerSupervisor;

/// 起動済み play server のポート。実サーバーテストの入口。
const REAL_SERVER_PORT_ENV: &str = "CMRT_LIVE_CACHE_TEST_PORT";
/// 鳴らすキャッシュ WAV の絶対パス。マシン固有なのでコードへ書かない。
const REAL_SERVER_WAV_ENV: &str = "CMRT_LIVE_CACHE_TEST_WAV";

/// 起動済みの実サーバーへ繋いだ supervisor と、鳴らすキャッシュ WAV。
///
/// 環境変数が揃っていなければ `None`（＝呼び出し側のテストは何もせず green）。
pub(super) fn real_server_from_env() -> Option<(RealtimePlayServerSupervisor, PathBuf)> {
    let (Ok(port), Ok(wav)) = (
        std::env::var(REAL_SERVER_PORT_ENV),
        std::env::var(REAL_SERVER_WAV_ENV),
    ) else {
        return None;
    };
    let port: u16 = port.parse().expect("ポートは数値");
    let wav = PathBuf::from(wav);
    assert!(wav.is_file(), "キャッシュ WAV が見つからない: {wav:?}");

    let cfg: cmrt_runtime::Config = toml::from_str(&format!(
        r#"
input_midi = "input.mid"
output_midi = "output.mid"
output_wav = "output.wav"
sample_rate = 48000
buffer_size = 512
realtime_play_server_port = {port}
realtime_audio_backend = "cache_player"
"#
    ))
    .expect("テスト用 config");
    Some((RealtimePlayServerSupervisor::new(&cfg), wav))
}
