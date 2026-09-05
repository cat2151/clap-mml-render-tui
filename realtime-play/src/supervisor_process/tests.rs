//! プロセス生存管理のテスト。実プロセスを起動するものはここに置く。

use std::{
    net::TcpListener,
    time::{Duration, Instant},
};

use super::PLAY_SERVER_START_TIMEOUT;
use cmrt_runtime::PlayServerLaunch;

use crate::{tests::cfg_for_port, RealtimePlayServerSupervisor, LIVE_INSTANCE_COUNT_ENV};

#[test]
fn an_injected_shell_command_is_reported_as_such() {
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(62_154));
    let launch = supervisor.build_command().unwrap();

    assert_eq!(
        launch.description,
        "source=shell-command profile=不明 fullpath=\"exit 0\""
    );
    // 落ちたときのエラーと UI は description ではなく実体だけを見せる。
    assert_eq!(launch.resolved.exe, "exit 0");
}

/// 実体がどこにも無いときは、素の実行ファイル名で spawn して OS のエラーに任せず、
/// **探した場所が分かるエラーで止める**（ADR 0017）。
///
/// テストバイナリは `target/debug/deps/` に居るので、隣にサーバーの実体は無く、
/// 兄弟 repo の経路（`target/<profile>/` 直下のときだけ効く）も成立しない。
#[test]
fn a_missing_server_binary_fails_with_the_places_it_looked() {
    let mut cfg = cfg_for_port(62_154);
    cfg.play_server_launch_override = None;
    let supervisor = RealtimePlayServerSupervisor::new(&cfg);

    let Err(error) = supervisor.build_command() else {
        panic!("実体が無いのに起動しようとしないこと");
    };

    let message = format!("{error:#}");
    assert!(message.contains("実体が見つかりません"), "{message}");
    assert!(message.contains("探した場所"), "{message}");
    assert!(
        supervisor.last_startup_failure().is_some(),
        "画面に出せるよう理由を残すこと"
    );
}

/// 即死するサーバーを指定したときは、30 秒粘らずに「なぜ落ちたか」を返す。
///
/// 古い server exe を掴んで config を拒否され続けた事故では、ポーリングのたびに
/// spawn し直して 1 セッション数百プロセスを作り、ユーザーに見えるのは無音だけだった。
#[test]
fn a_server_that_exits_immediately_fails_fast_with_the_reason() {
    let port = free_port();
    let mut cfg = cfg_for_port(port);
    cfg.play_server_launch_override = Some(PlayServerLaunch::ShellCommand(
        immediately_failing_command().to_string(),
    ));
    let supervisor = RealtimePlayServerSupervisor::new(&cfg);

    let started = Instant::now();
    let error = supervisor.ensure_started_for_fast_midi().unwrap_err();

    let message = format!("{error:#}");
    assert!(
        message.contains("Error: boom"),
        "子が言い残した理由をそのまま返すこと: {message}"
    );
    assert!(
        message.contains(immediately_failing_command()),
        "どの実体を起動したのかを言うこと: {message}"
    );
    assert!(
        started.elapsed() < PLAY_SERVER_START_TIMEOUT,
        "起動タイムアウトを待たずに諦めること"
    );

    let failure = supervisor.last_startup_failure().expect("落ちた理由が残る");
    let crate::ServerStartupFailure::Exited {
        exit_code,
        stderr_tail,
        ..
    } = failure
    else {
        panic!("起動はして落ちたことを区別すること: {failure:?}");
    };
    assert_eq!(exit_code, Some(3));
    // cmd の echo は `1>&2` の手前の空白ごと出すので、比較は trim してから。
    assert_eq!(
        stderr_tail
            .iter()
            .map(|line| line.trim())
            .collect::<Vec<_>>(),
        vec!["Error: boom"]
    );
}

/// 打ち切ったあとは、同じ理由のまま spawn を繰り返さない。
#[test]
fn a_latched_supervisor_stops_spawning_more_processes() {
    let port = free_port();
    let mut cfg = cfg_for_port(port);
    cfg.play_server_launch_override = Some(PlayServerLaunch::ShellCommand(
        immediately_failing_command().to_string(),
    ));
    let supervisor = RealtimePlayServerSupervisor::new(&cfg);

    supervisor.ensure_started_for_fast_midi().unwrap_err();
    let started = Instant::now();
    let error = supervisor.ensure_started_for_fast_midi().unwrap_err();

    assert!(
        started.elapsed() < Duration::from_secs(1),
        "打ち切り後は spawn せずに即答すること"
    );
    assert!(format!("{error:#}").contains("Error: boom"));
}

/// stderr へ 1 行残してから失敗するコマンド。shell 経由で起動される前提。
fn immediately_failing_command() -> &'static str {
    if cfg!(windows) {
        "echo Error: boom 1>&2& exit 3"
    } else {
        "echo 'Error: boom' 1>&2; exit 3"
    }
}

/// 誰も listen していないポートを取る。
fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

#[test]
fn supervisor_keeps_requested_live_instance_count() {
    let supervisor =
        RealtimePlayServerSupervisor::with_live_instance_count(&cfg_for_port(62_154), 4);
    assert_eq!(supervisor.live_instance_count(), 4);
    let launch = supervisor.build_command().unwrap();
    assert_eq!(
        launch
            .command
            .get_envs()
            .find(|(name, _)| *name == LIVE_INSTANCE_COUNT_ENV)
            .and_then(|(_, value)| value)
            .and_then(std::ffi::OsStr::to_str),
        Some("4")
    );
}

#[test]
fn owned_start_refuses_an_already_listening_port() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    let error = supervisor.start_owned_for_fast_midi().unwrap_err();

    assert!(format!("{error:#}").contains("既に使用中"));
}
