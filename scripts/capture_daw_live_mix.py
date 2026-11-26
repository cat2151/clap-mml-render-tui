#!/usr/bin/env python3
"""DAW の live mix が実際に出している音を 1 本の WAV へ録り、拍の位置を測る。

    python scripts/capture_daw_live_mix.py
    python scripts/capture_daw_live_mix.py --loop-measures 4 --measures 12
    python scripts/capture_daw_live_mix.py --analyse-only   # 録らずに測るだけ
    python scripts/capture_daw_live_mix.py --only-row 6      # hi-hat だけを録る

## なぜこれが要るか

`verify_daw_playback_timing.py` は「いつ予約したか」「サーバーが late と言ったか」を
測る。どちらも**送り手側の帳簿**で、実際に出た音は見ていない。素材（キャッシュ WAV）を
1 本ずつ調べても同じで、そこには鳴らし方の誤りは写らない。

「素材は正しい・予約も正しい・なのにモタって聴こえる」を切り分けられるのは、
**混ざったあとの波形**だけ。このスクリプトは:

1. release ビルドの play server を `CMRT_LIVE_CAPTURE_WAV` 付きで起動する
2. ユーザーの実キャッシュで演奏ループを丸ごと走らせる
   （`daw/src/playback/live_cache/tests/capture.rs`）
3. サーバーを落として、録れた WAV を書き出させる
4. **期待波形を再構成して、録れた波形と相互相関で突き合わせる**

出力の WAV はそのまま聴ける。まだモタって聴こえるなら、そのファイルを分析すればよい。

## 判定の芯（`lag`）

小節ごとに「その小節のキャッシュ WAV を全 track 足したもの」を作り、録れた波形の
**予約位置のまわりで相互相関を取る**。ピークの位置 `lag` が 0 なら、予約したとおりの
サンプルで、素材どおりの中身が鳴っている。0 でなければ、その量がそのまま
「その小節が何サンプル遅れて（進んで）鳴ったか」になる。

`corr` は正規化相関のピーク値。低いときは「そもそも別物が鳴っている」ということで、
lag の値には意味が無い（素材とスケジュールの取り違え、鳴らす小節の間違いなど）。

## 1 行だけ録る（`--only-row`）

7 本が混ざった波形では、小節を 1 つの塊として合わせる `lag` までしか読めない。
**hi-hat のようにアタックが鋭く等間隔な行を 1 本だけ録る**と、打点を 1 つずつ
拾って「小節の中は合っているのに切り替わりだけ間隔が違う」まで見える
（`daw_live_attacks.py`）。

`--only-row` は **鳴らす行を減らさない。** mixer で他の行を -120dB へ落とすだけで、
state load も note on も全行ぶん出る。ロードの重さはこの不具合の当事者なので、
そこを軽くしてしまうと実演奏と違う条件を測ることになる。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

from daw_live_mix_analysis import analyse
from daw_live_schedule import (
    SAMPLE_RATE,
    check_origin_alignment,
    check_slot_overwrites,
    parse_schedule,
)

TUI_ROOT = Path(__file__).resolve().parent.parent
PLAY_SERVER_ROOT = TUI_ROOT.parent / "clap-mml-play-server"
SERVER_EXE_NAME = "clap-mml-realtime-play-server.exe"
DEFAULT_PORT = 8714
BEATS_PER_MEASURE = 4


def default_cache_dir() -> Path:
    local = os.environ.get("LOCALAPPDATA", "")
    return Path(local) / "clap-mml-render-tui" / "daw_cache" / "Surge XT" / "daily"


def server_exe(profile: str) -> Path:
    """使う play server の実体を直に指す。

    PATH 上の古い版が使われて「直したはずの挙動が変わらない」事故を避けるため、
    ここは必ず `../clap-mml-play-server/target/<profile>/` を見る。
    """
    return PLAY_SERVER_ROOT / "target" / profile / SERVER_EXE_NAME


def start_server(
    exe: Path,
    port: int,
    instances: int,
    log_path: Path,
    capture_wav: Path,
    seconds: float,
) -> subprocess.Popen:
    env = dict(os.environ)
    env["CMRT_REALTIME_PLAY_SERVER_PORT"] = str(port)
    env["CMRT_LIVE_INSTANCE_COUNT"] = str(instances)
    env["CMRT_LIVE_CAPTURE_WAV"] = str(capture_wav)
    env["CMRT_LIVE_CAPTURE_SECONDS"] = str(seconds)
    log = log_path.open("w", encoding="utf-8", errors="replace")
    return subprocess.Popen(
        [str(exe)],
        cwd=str(PLAY_SERVER_ROOT),
        env=env,
        stdout=log,
        stderr=subprocess.STDOUT,
    )


def stop_server(process: subprocess.Popen) -> None:
    """起動したサーバーを子ごと確実に落とす。

    孤児になった play server は SHM を握って次回の起動を壊すので、例外が出ても
    必ず通ること（finally から呼ぶ）。録れた WAV の書き出しは演奏停止の時点で
    済んでいるので、ここで強制終了しても失わない。
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
        process.wait(timeout=15)
    except subprocess.TimeoutExpired:
        process.kill()


def run_capture_test(
    port: int,
    cache_dir: Path,
    tracks: int,
    measures: int,
    bpm: float,
    loop_measures: int,
    gain_db: int,
    only_row: int,
    start_measure: int,
) -> tuple[int, str]:
    env = dict(os.environ)
    env["CMRT_LIVE_CACHE_TEST_PORT"] = str(port)
    env["CMRT_LIVE_CACHE_CAPTURE_DIR"] = str(cache_dir)
    env["CMRT_LIVE_CACHE_CAPTURE_TRACKS"] = str(tracks)
    env["CMRT_LIVE_CACHE_CAPTURE_MEASURES"] = str(measures)
    env["CMRT_LIVE_CACHE_CAPTURE_BPM"] = str(bpm)
    env["CMRT_LIVE_CACHE_CAPTURE_GAIN_DB"] = str(gain_db)
    env["CMRT_LIVE_CACHE_CAPTURE_START_MEASURE"] = str(start_measure)
    if only_row:
        # 鳴らす行は減らさない（state load の重さを変えないため）。聴こえる行だけを絞る。
        env["CMRT_LIVE_CACHE_CAPTURE_ONLY_ROW"] = str(only_row)
    if loop_measures:
        env["CMRT_LIVE_CACHE_CAPTURE_LOOP"] = str(loop_measures)
    command = [
        "cargo",
        "test",
        "-p",
        "cmrt-daw",
        "--lib",
        "live_cache::tests::capture",
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
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return result.returncode, result.stdout


CAPTURE_WRITTEN_LINE = re.compile(
    r"^cmrt-live-capture: event=written .*? first_clock=(\d+)", re.MULTILINE
)


def parse_first_clock(server_log: Path) -> int:
    """録音の 1 フレーム目にあたるサンプルクロック。

    `at_frames` はサーバーのクロックの絶対位置、録れた WAV の位置は
    「録り始めてから」。**両者を突き合わせるにはこの差を引く。** 実測では
    live の最初のブロックから録れているので 0 だが、0 だと決め打ちにすると
    「固定オフセットが出たとき、録音の原点のせいなのか演奏のせいなのか」が
    切り分けられなくなる。だからログから読む。
    """
    if not server_log.is_file():
        return 0
    found = CAPTURE_WRITTEN_LINE.search(server_log.read_text(encoding="utf-8", errors="replace"))
    return int(found.group(1)) if found else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument("--instances", type=int, default=8)
    parser.add_argument(
        "--tracks",
        type=int,
        default=9,
        help="DAW の track 数。鳴る行は 2..tracks なので、**実演奏で 7 行鳴っているなら 9**。"
        "ここを実演奏より小さくすると鳴る行が減り、state load の数も減って別条件になる",
    )
    parser.add_argument("--measures", type=int, default=8, help="録る小節数")
    parser.add_argument("--bpm", type=float, default=113.0)
    parser.add_argument(
        "--loop-measures",
        type=int,
        default=0,
        help="ループの長さ（小節数）。0 ならキャッシュにある数。**実演奏の "
        "effective_count と揃えること**（古い BPM で焼かれた小節まで鳴らすと、"
        "実演奏には無いモタりを録ってしまう）",
    )
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=default_cache_dir(),
        help="実キャッシュのディレクトリ（track<行>_meas<小節>.wav が並ぶところ）",
    )
    parser.add_argument("--server-profile", choices=("debug", "release"), default="release")
    parser.add_argument(
        "--out",
        type=Path,
        default=TUI_ROOT / "daw-live-mix-capture.wav",
        help="録れた live mix の書き出し先",
    )
    parser.add_argument(
        "--gain-db",
        type=int,
        default=-12,
        help="1 track あたりの gain。0dB のまま足すと master limiter で潰れて、"
        "包絡から小節の頭が読めなくなる",
    )
    parser.add_argument(
        "--only-row",
        type=int,
        default=0,
        help="この行だけを聴こえる状態にして録る（グリッドの行番号。hi-hat は実測で 6）。"
        "**鳴らす行は減らさない**（mixer で他を -120dB へ落とすだけ）ので、state load の"
        "重さは全行のときと同じまま。指定するとアタック位置の一覧が出る",
    )
    parser.add_argument(
        "--start-measure",
        type=int,
        default=1,
        help="演奏を始める小節（1 始まり）。実アプリの「カーソルの小節から演奏」と同じ入口で、"
        "1 以外にすると 1 小節目が preload=miss になりグリッドを張り直す経路を通る。"
        "**ループ長 5 で 5 を指定すると、末尾と先頭が同じスロットになって踏み潰しが出る**",
    )
    parser.add_argument("--analyse-only", action="store_true", help="録らずに測るだけ")
    args = parser.parse_args()

    measure_frames = round(BEATS_PER_MEASURE * 60.0 / args.bpm * SAMPLE_RATE)
    schedule_path = args.out.with_suffix(".schedule.json")

    server_log = TUI_ROOT / "daw-live-mix-capture-server.log"

    if args.analyse_only:
        schedule = json.loads(schedule_path.read_text()) if schedule_path.is_file() else []
        origin_ok = check_origin_alignment(schedule, measure_frames)
        slots_ok = check_slot_overwrites(server_log, schedule, measure_frames)
        attacks_ok = analyse(
            args.out,
            schedule,
            args.cache_dir,
            args.tracks,
            measure_frames,
            args.only_row,
            parse_first_clock(server_log),
        )
        return 0 if slots_ok and origin_ok and attacks_ok else 1

    exe = server_exe(args.server_profile)
    if not exe.is_file():
        print(f"play server が無い: {exe}", file=sys.stderr)
        return 1
    if not args.cache_dir.is_dir():
        print(f"キャッシュのディレクトリが無い: {args.cache_dir}", file=sys.stderr)
        return 1

    args.out.unlink(missing_ok=True)
    seconds = measure_frames / SAMPLE_RATE * (args.measures + 4)
    print(f"server: {exe}")
    print(f"capture: {args.out} ({seconds:.1f}s まで)")
    server = start_server(exe, args.port, args.instances, server_log, args.out, seconds)
    try:
        code, output = run_capture_test(
            args.port,
            args.cache_dir,
            args.tracks,
            args.measures,
            args.bpm,
            args.loop_measures,
            args.gain_db,
            args.only_row,
            args.start_measure,
        )
        for line in output.splitlines():
            if "live-cache" in line or "live-capture" in line or "test result" in line:
                print(line)
    finally:
        stop_server(server)

    schedule = parse_schedule(output)
    schedule_path.write_text(json.dumps(schedule, indent=2))

    if server_log.is_file():
        for line in server_log.read_text(encoding="utf-8", errors="replace").splitlines():
            if "cmrt-live-capture" in line:
                print(line)

    origin_ok = check_origin_alignment(schedule, measure_frames)
    slots_ok = check_slot_overwrites(server_log, schedule, measure_frames)

    if not args.out.is_file():
        print(f"!! 録れていない: {args.out}", file=sys.stderr)
        print(f"   サーバーログを見ること: {server_log}", file=sys.stderr)
        return 1
    attacks_ok = analyse(
        args.out,
        schedule,
        args.cache_dir,
        args.tracks,
        measure_frames,
        args.only_row,
        parse_first_clock(server_log),
    )
    print()
    print(f"聴いて確かめるファイル: {args.out}")
    # 原点・上書き・刻みの判定が落ちたら、録れたかどうかに関わらず失敗として返す。
    return code or (0 if slots_ok and origin_ok and attacks_ok else 1)


if __name__ == "__main__":
    raise SystemExit(main())
