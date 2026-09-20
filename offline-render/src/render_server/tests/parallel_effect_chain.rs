//! 同一プロセス内の複数スレッドが **1 つの `RenderServerSupervisor` を共有**して、
//! effect chain 付きの MML を並列に render する（DAW の `offline_render_server_workers` と同じ形）。
//!
//! 実 render-server と実 plugin（TONE3000）が要るので `#[ignore]`。render-server の実体は
//! 環境変数で受ける（テストバイナリは `target/debug/deps/` に居るので兄弟 repo の探索が効かない）:
//!
//! ```text
//! CMRT_TEST_RENDER_SERVER_EXE=<clap-mml-play-server>/target/release/clap-mml-render-server.exe
//! CMRT_TEST_PARALLEL_RENDER_INIT_JSON={"Surge XT patch":"<音色>","effects after instrument":[{"TONE3000 preset":"Bogner Fullstack"}]}
//! cargo test -p cmrt-offline-render --release render_server::tests::parallel_effect_chain -- --ignored --nocapture
//! ```
//!
//! render-server 自身は既定の config.toml（plugin のパス・port）を読む。port は config の
//! 既定値をそのまま使うので、TUI が起動中（render-server が listen 中）なら走らせない。
//!
//! 失敗したときは、子プロセスが生きているか（`server-exited` が exit code つきで出るか）まで
//! 見てから終わる。transport error と同時には exit status が取れないことがあるため。

use std::{
    sync::{Arc, OnceLock},
    time::Instant,
};

use super::super::*;
use crate::set_log_sink;

const RENDER_SERVER_EXE_ENV: &str = "CMRT_TEST_RENDER_SERVER_EXE";
/// 各 measure の先頭 JSON（音色と chain）。既定は「既定 plugin + TONE3000」。
/// 音色によって結果が変わるので、DAW の init セルと同じ JSON をここで差し替えられるようにする。
const INIT_JSON_ENV: &str = "CMRT_TEST_PARALLEL_RENDER_INIT_JSON";
const DEFAULT_INIT_JSON: &str =
    r#"{"effects after instrument":[{"TONE3000 preset":"Bogner Fullstack"}]}"#;
const MEASURES: [&str; 2] = ["t120 o4 l8 cdefgab>c", "t120 o5 l8 c<bagfedc"];

static STARTED: OnceLock<Instant> = OnceLock::new();

fn print_log_line(line: &str) {
    eprintln!(
        "[{:>7} ms] {line}",
        STARTED.get_or_init(Instant::now).elapsed().as_millis()
    );
}

fn supervisor_for_real_server(exe: &str) -> RenderServerSupervisor {
    let cfg: Config = toml::from_str(&format!(
        r#"
plugin_path = "dummy.clap"
input_midi = "input.mid"
output_midi = "output.mid"
output_wav = "output.wav"
sample_rate = 48000
buffer_size = 512
offline_render_server_command = {exe:?}
"#
    ))
    .unwrap();
    assert!(
        TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], cfg.offline_render_server_port)),
            RENDER_SERVER_CONNECT_TIMEOUT
        )
        .is_err(),
        "port {} で既に何かが listen している（TUI 起動中？）ので、このテストは走らせない",
        cfg.offline_render_server_port
    );
    RenderServerSupervisor::new(&cfg)
}

#[test]
#[ignore = "実 render-server と TONE3000 が要る。CMRT_TEST_RENDER_SERVER_EXE を設定して --ignored で走らせる"]
fn two_threads_render_tone3000_chains_through_one_supervisor() {
    let exe = std::env::var(RENDER_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{RENDER_SERVER_EXE_ENV} に render-server の実体を設定すること")
    });
    let init_json = std::env::var(INIT_JSON_ENV).unwrap_or_else(|_| DEFAULT_INIT_JSON.to_string());
    STARTED.get_or_init(Instant::now);
    set_log_sink(print_log_line);
    print_log_line(&format!("test: init_json={init_json}"));
    // 1 回目の transport error で止める（kill と再起動の無限ループを避け、失敗の直接証拠を残す）。
    let mut supervisor = supervisor_for_real_server(&exe);
    supervisor.set_never_kill_for_test(true);
    let supervisor = Arc::new(supervisor);

    let workers = MEASURES
        .iter()
        .map(|body| {
            let supervisor = Arc::clone(&supervisor);
            let mml = format!("{init_json}{body}");
            std::thread::spawn(move || {
                let started = Instant::now();
                let result = supervisor.render_mml(&mml);
                (mml, started.elapsed(), result)
            })
        })
        .collect::<Vec<_>>();

    let mut failures = Vec::new();
    for worker in workers {
        let (mml, elapsed, result) = worker.join().expect("render thread panicked");
        match result {
            Ok(output) => print_log_line(&format!(
                "test: mml={mml} elapsed_ms={} samples={}",
                elapsed.as_millis(),
                output.samples.len()
            )),
            Err(error) => {
                print_log_line(&format!(
                    "test: mml={mml} elapsed_ms={} error={error:#}",
                    elapsed.as_millis()
                ));
                failures.push(format!("{mml}: {error:#}"));
            }
        }
    }
    if !failures.is_empty() {
        std::thread::sleep(Duration::from_secs(3));
        supervisor.log_child_status_for_test();
    }
    assert!(
        failures.is_empty(),
        "並列 render が失敗した:\n{}",
        failures.join("\n")
    );
}
