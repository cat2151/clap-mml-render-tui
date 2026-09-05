#!/usr/bin/env python3
"""DAW 演奏の「モタり」「ぶつ切り」が直っていることを、1 コマンドで測り直す。

docs/adr/0016-daw-live-playback-slots-and-timeline.md の統合検証。実サーバーへ繋ぐテストは
「起動済みのサーバーへ環境変数でポートと WAV を渡す」形になっていて、手で並べると
6 手ほど掛かる。**その手順をここへ閉じて、繰り返し実行できるようにする。**

    python scripts/verify_daw_playback_timing.py
    python scripts/verify_daw_playback_timing.py --server-profile release

やること:

1. play server（デバッグビルド）を専用ポートで起動する
2. キャッシュ WAV を temp へ生成する（マシン固有のパスをコードへ書かないため）
3. `cargo test -p cmrt-daw --lib live_cache::tests` を実サーバー付きで走らせる
4. 小節ログを拾って `at_frames` の間隔・`prepare_ms` / `next_ms` を表にして出す
5. **起動したサーバーを必ず落とす**（孤児が SHM を握ると次回起動が壊れる）

判定そのものはテスト側の assert が持っている（`tests/jitter.rs` の
「小節長ちょうど＋ late 0 件」、`tests/state_load.rs` / `tests/play_loop.rs` の
「境界の prepare_ms が 0」）。ここが出す表は**その数値を人が読める形にするだけ**で、
green/red を決めるのは cargo test の終了コード。

`--server-profile release` を付けると、ユーザーが実際に使う release ビルドの
サーバーで測る。debug は WAV デコードが 6〜7 倍重く、先読みの最中に音が途切れる
（`dropouts=` の列に出る）ので、**「途切れが無い」を見たいときは release で走らせること。**
"""

from __future__ import annotations

import argparse
import math
import os
import re
import socket
import struct
import subprocess
import sys
import tempfile
import time
import wave
from pathlib import Path

TUI_ROOT = Path(__file__).resolve().parent.parent
PLAY_SERVER_ROOT = TUI_ROOT.parent / "clap-mml-play-server"
SERVER_EXE_NAME = "clap-mml-realtime-play-server.exe"


def server_exe(profile: str) -> Path:
    """使う play server の実体。

    **どの実体が動いているかを取り違えないこと。** 過去に「PATH 上の古い版が
    使われていて、直したはずの挙動が変わらない」事故がある。ここは必ず
    `../clap-mml-play-server/target/<profile>/` を直に指す。
    """
    return PLAY_SERVER_ROOT / "target" / profile / SERVER_EXE_NAME

# 既定ポート。TUI が普段使うポートとぶつけないために別の番号を使う。
DEFAULT_PORT = 8713
# 生成するキャッシュ WAV の長さ（秒）。実キャッシュ（小節 2.4 秒に対して約 4.1 秒）と
# 同じく**小節長より長く**して、余韻が次の小節へはみ出す条件を再現する。
WAV_SECONDS = 4.2
SAMPLE_RATE = 48_000


def generate_cache_wav(path: Path) -> Path:
    """減衰する正弦波のステレオ WAV を書く。

    実キャッシュの代わり。中身は何でもよいが、無音だと「本当に鳴っているか」が
    サーバーログの auto gain から読めなくなるので、ちゃんと振幅を持たせる。
    """
    frames = int(WAV_SECONDS * SAMPLE_RATE)
    with wave.open(str(path), "wb") as out:
        out.setnchannels(2)
        out.setsampwidth(2)
        out.setframerate(SAMPLE_RATE)
        samples = bytearray()
        for frame in range(frames):
            t = frame / SAMPLE_RATE
            envelope = math.exp(-t * 1.5)
            left = int(12000 * envelope * math.sin(2 * math.pi * 440.0 * t))
            right = int(12000 * envelope * math.sin(2 * math.pi * 660.0 * t))
            samples += struct.pack("<hh", left, right)
        out.writeframes(bytes(samples))
    return path


def port_is_open(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.settimeout(0.2)
        return probe.connect_ex(("127.0.0.1", port)) == 0


def wait_for_server(port: int, timeout_s: float) -> bool:
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        if port_is_open(port):
            return True
        time.sleep(0.2)
    return False


def start_server(exe: Path, port: int, instances: int, log_path: Path) -> subprocess.Popen:
    env = dict(os.environ)
    env["CMRT_REALTIME_PLAY_SERVER_PORT"] = str(port)
    env["CMRT_LIVE_INSTANCE_COUNT"] = str(instances)
    log = log_path.open("w", encoding="utf-8", errors="replace")
    return subprocess.Popen(
        [str(exe)],
        cwd=str(PLAY_SERVER_ROOT),
        env=env,
        stdout=log,
        stderr=subprocess.STDOUT,
    )


def stop_server(process: subprocess.Popen) -> None:
    """起動したサーバーを、子プロセスごと確実に落とす。

    孤児になった play server は SHM を握り、次回の起動を壊す。この関数は
    テストが失敗しても例外が出ても必ず通ること（finally から呼ぶ）。
    """
    if process.poll() is not None:
        return
    subprocess.run(
        ["taskkill", "/PID", str(process.pid), "/T", "/F"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()


def run_tests(port: int, wav: Path, tracks: int, filter_: str) -> tuple[int, str]:
    env = dict(os.environ)
    env["CMRT_LIVE_CACHE_TEST_PORT"] = str(port)
    env["CMRT_LIVE_CACHE_TEST_WAV"] = str(wav)
    env["CMRT_LIVE_CACHE_TEST_TRACKS"] = str(tracks)
    command = [
        "cargo",
        "test",
        "-p",
        "cmrt-daw",
        "--lib",
        filter_,
        "--",
        "--test-threads=1",
        "--nocapture",
    ]
    print("$ " + " ".join(command), flush=True)
    result = subprocess.run(
        command,
        cwd=str(TUI_ROOT),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        encoding="utf-8",
        errors="replace",
    )
    print(result.stdout, flush=True)
    return result.returncode, result.stdout


MEASURE_LINE = re.compile(r"(meas\d+): live-cache (.*)")


def field(fields: str, name: str) -> str:
    match = re.search(r"\b" + name + r"=(\S+)", fields)
    return match.group(1) if match else "-"


def summarise(output: str) -> None:
    """テスト出力の小節ログを表にする。判定ではなく、数値を目で並べるため。"""
    rows = []
    for line in output.splitlines():
        match = MEASURE_LINE.search(line)
        if match:
            rows.append((match.group(1), match.group(2)))
    if not rows:
        print("小節ログが 1 行も出ていない（実サーバーテストが skip された可能性）")
        return

    print("")
    print("=== 小節ログ（Stage 6 実測） ===")
    header = "meas   slot  preload     at_frames     diff  prepare_ms  note_on_ms  next_ms"
    print(header)
    previous = None
    for name, fields in rows:
        at_frames = field(fields, "at_frames")
        diff = ""
        if at_frames != "-":
            current = int(at_frames)
            if previous is not None:
                diff = str(current - previous)
            previous = current
        print(
            "{:<7}{:<6}{:<12}{:>10}{:>9}{:>12}{:>12}{:>9}".format(
                name,
                field(fields, "slot"),
                field(fields, "preload"),
                at_frames,
                diff,
                field(fields, "prepare_ms"),
                field(fields, "note_on_ms"),
                field(fields, "next_ms"),
            )
        )

    for line in output.splitlines():
        if "live-cache jitter:" in line:
            print("")
            print(line.strip())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument(
        "--instances",
        type=int,
        default=8,
        help="play server の live instance 数（CMRT_LIVE_INSTANCE_COUNT）",
    )
    parser.add_argument(
        "--tracks", type=int, default=7, help="state_load テストが鳴らす演奏 track 数"
    )
    parser.add_argument(
        "--filter", default="live_cache::tests", help="cargo test へ渡すテスト名フィルタ"
    )
    parser.add_argument(
        "--server-profile",
        choices=("debug", "release"),
        default="debug",
        help="使う play server のビルド。debug は WAV デコードが release の 6〜7 倍重く、"
        "先読みの最中に音が途切れる。ユーザーが実際に使うのは release",
    )
    args = parser.parse_args()

    # cargo も小節ログも UTF-8 で出る。既定の cp932 コンソールへそのまま流すと
    # UnicodeEncodeError で落ちるので、この 1 か所で出力側を UTF-8 へ倒す。
    for stream in (sys.stdout, sys.stderr):
        stream.reconfigure(encoding="utf-8", errors="replace")

    exe = server_exe(args.server_profile)
    if not exe.is_file():
        print("play server の {} ビルドが無い: {}".format(args.server_profile, exe))
        build = "cargo build" if args.server_profile == "debug" else "cargo build --release"
        print("  cd " + str(PLAY_SERVER_ROOT) + " && " + build)
        return 2
    if port_is_open(args.port):
        print(
            "ポート {} が既に使われている。別プロセスの play server が"
            "生きている可能性がある（--port で変えられる）".format(args.port)
        )
        return 2

    workdir = Path(tempfile.mkdtemp(prefix="cmrt-daw-timing-"))
    wav = generate_cache_wav(workdir / "cache.wav")
    server_log = workdir / "server.log"
    print("作業ディレクトリ: " + str(workdir))

    print("play server: " + str(exe))
    server = start_server(exe, args.port, args.instances, server_log)
    code = 2
    try:
        if not wait_for_server(args.port, timeout_s=120.0):
            print("play server がポート {} で待ち受けない。ログ: {}".format(args.port, server_log))
            return 2
        code, output = run_tests(args.port, wav, args.tracks, args.filter)
        summarise(output)
    finally:
        stop_server(server)
        print("")
        print("play server を停止した（ログ: " + str(server_log) + "）")

    print("")
    print("=== サーバー側 timing（" + args.server_profile + " ビルド） ===")
    for line in server_log.read_text(encoding="utf-8", errors="replace").splitlines():
        if "cmrt-timing:" in line:
            print(line.strip())

    return code


if __name__ == "__main__":
    sys.exit(main())
