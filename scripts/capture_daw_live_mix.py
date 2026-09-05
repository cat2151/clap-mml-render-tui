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
import math
import os
import re
import struct
import subprocess
import sys
from pathlib import Path

import numpy as np

import daw_live_attacks
import daw_live_material_match

TUI_ROOT = Path(__file__).resolve().parent.parent
PLAY_SERVER_ROOT = TUI_ROOT.parent / "clap-mml-play-server"
SERVER_EXE_NAME = "clap-mml-realtime-play-server.exe"
DEFAULT_PORT = 8714
SAMPLE_RATE = 48_000
BEATS_PER_MEASURE = 4
FIRST_PLAYABLE_ROW = 2
# 相互相関で探す範囲（フレーム）。小節長の 1/4 も探せば充分で、これ以上広げると
# 隣の小節のピークを拾ってしまう。
SEARCH_FRAMES = 12_000


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


MEASURE_LINE = re.compile(
    r"^meas(\d+): live-cache .*? at_frames=(\d+) prepare_ms=([\d.]+)", re.MULTILINE
)


def parse_schedule(output: str) -> list[dict]:
    """小節ログから `(小節番号, 予約したフレーム位置, 境界で待たされた ms)` を取り出す。

    これが「どの素材がどこで鳴るはずだったか」の正解表になる。録れた波形と
    突き合わせる相手はここから作る。`prepare_ms` は
    [`check_origin_alignment`] が使う（1 小節目のロードにかかった実時間）。
    """
    return [
        {
            "measure": int(m.group(1)),
            "at_frames": int(m.group(2)),
            "prepare_ms": float(m.group(3)),
        }
        for m in MEASURE_LINE.finditer(output)
    ]


# 演奏ループが「いまから鳴らす」ときに空ける先行時間（`timeline.rs` の `MAX_LEAD`）。
# 小節が短いときは小節長の半分まで縮む、という縮め方まで同じにしてある。
MAX_LEAD_FRAMES = 12_000
# 1 小節目の `at_frames` が先行時間からずれてよい上限（フレーム = 50ms）。
# 正しい順序なら「クロックを起こす → すぐ予約する」だけなので実測は 0 フレーム。
# ここに OS のスケジューラのぶれぶんだけ余裕を持たせてある。
ORIGIN_TOLERANCE_FRAMES = 2_400


def check_origin_alignment(schedule: list[dict], measure_frames: int) -> bool:
    """**timeline の原点とサーバーのクロックの原点が揃っているか。**

    ここが `docs/adr/0012-live-clock-drift-is-absorbed-not-eliminated.md` の
    「開始時の原点合わせ（案 A）」の判定。
    スロット上書きの判定（[`check_slot_overwrites`]）と違い、**1 小節目のロードが
    速い日でも成り立つ**のがこの判定の値打ち。

    上書きの判定は「ロードにかかった実時間ぶんの先行」が 1 小節（約 2 秒）を
    食い潰すほど大きくないと赤くならない。ロードの実測は**OS のファイルキャッシュ次第で
    3384ms にも 107ms にもなる**ので、暖まった状態では順序を壊しても緑のままになる。

    順序そのものは `at_frames` を見れば一発で分かる:

    - 正しい順（ロード → クロック起こし → 予約）なら、1 小節目の `at_frames` は
      先行時間ちょうど（実測 12000）。ロードに何秒かかっても変わらない
    - 壊れた順（クロック起こし → ロード → 予約）なら、そこへ**ロードにかかった
      時間ぶんのフレーム数が上乗せされる**。これが演奏ループの先行そのもの
    """
    if not schedule:
        print("!! 小節ログが無いので原点の判定を飛ばす", file=sys.stderr)
        return False
    first = schedule[0]
    lead = min(MAX_LEAD_FRAMES, measure_frames // 2)
    drift = first["at_frames"] - lead
    prepare_ms = first.get("prepare_ms")
    prepare_frames = round(prepare_ms / 1000 * SAMPLE_RATE) if prepare_ms is not None else None
    print()
    print("== 原点の判定（1 小節目の at_frames に、ロードの時間が乗っていないか）")
    print(f"   at_frames={first['at_frames']} lead={lead} drift={drift} frames"
          f" ({drift / SAMPLE_RATE * 1000:.1f}ms)")
    if prepare_frames is not None:
        print(f"   1 小節目のロード: prepare_ms={prepare_ms} ({prepare_frames} frames)")
    # 絶対値だけで見ると、ロードが速い日（暖まったキャッシュで 107ms）に壊れた順序を
    # 見逃す。**ロード時間の何割が原点へ漏れたか**でも見る。壊れた順序では
    # drift ≒ prepare_frames になるので、ロードがどれだけ速くても赤くなる。
    leaked = prepare_frames is not None and drift > prepare_frames // 2
    ok = drift <= ORIGIN_TOLERANCE_FRAMES and not leaked
    if ok:
        print("   OK: クロックを起こした瞬間が原点になっている（ロードの前に起こしていない）")
    else:
        reason = (
            "ロード時間の半分より大きいぶん原点へ漏れている"
            if leaked
            else f"許容 {ORIGIN_TOLERANCE_FRAMES} frames を超えている"
        )
        print(f"   NG: 原点がロードのぶん先行している（{reason}）。クロックの起こしがロードより前に出ている")
    return ok


# サーバーが「スロットの中身を差し替えた瞬間の再生位置」を出す行
# （play server 側 `worker/command.rs` の `PrepareLivePatch`）。
LIVE_PATCH_LINE = re.compile(
    r"^cmrt-live-patch: event=apply instance=(\d+) clock=(\d+) patch=slot=(\d+);.*?_meas(\d+)\.wav",
    re.MULTILINE,
)


def parse_slot_loads(server_log_text: str) -> list[dict]:
    """サーバーログから「スロット S へ小節 M を載せた」ひとまとまりを取り出す。

    1 小節ぶんのロードは track 数ぶんの行に分かれて出る（instance ごと）。
    **効くのは最初の 1 行**（その瞬間からスロットの中身が変わり始める）なので、
    連続する同じ `(slot, measure)` の行はまとめて、いちばん小さい clock を代表にする。
    """
    groups: list[dict] = []
    for m in LIVE_PATCH_LINE.finditer(server_log_text):
        clock, slot, measure = int(m.group(2)), int(m.group(3)), int(m.group(4))
        if groups and groups[-1]["slot"] == slot and groups[-1]["measure"] == measure:
            groups[-1]["instances"] += 1
            continue
        groups.append({"slot": slot, "measure": measure, "clock": clock, "instances": 1})
    return groups


def check_slot_overwrites(
    server_log: Path, schedule: list[dict], measure_frames: int = 0
) -> bool:
    """**スロットが、その中身の note on が鳴る前に上書きされていないか。**

    これが `docs/adr/0012-live-clock-drift-is-absorbed-not-eliminated.md` の症状 B で、
    「違う小節が鳴る」の直接の原因。判定はログどうしの突き合わせだけで閉じるので、耳も波形も要らない。

    - 「slot S へ meas M を載せた clock」（サーバー側の実クロック）
    - 「その直前に slot S に載っていた小節の `at_frames`」（TUI 側の予約位置）

    後者のほうが大きければ、**まだ鳴っていない小節を踏み潰している**。
    `margin` はその差で、正なら安全・負なら踏み潰し。

    `measure_frames` を渡すと、**いちばん薄いところの余裕が何小節ぶんあるか**まで出す。
    これが Stage 5（スロットを 2 → 4 本）で買った保険の厚みそのもので、
    「別スロットへ逃げた」ことが数で見える（2 本のときは約 1 小節、4 本なら約 3 小節）。
    """
    if not server_log.is_file():
        print("!! サーバーログが無いのでスロット上書きの判定を飛ばす", file=sys.stderr)
        return True
    groups = parse_slot_loads(server_log.read_text(encoding="utf-8", errors="replace"))
    print()
    print("== スロット上書きの判定（margin が負なら、鳴る前に中身を差し替えている）")
    if not groups:
        print("   cmrt-live-patch の行が無い（サーバーが古いか、演奏していない）")
        return False
    print("      #  slot  meas   load_clock  prev_meas  prev_at_frames    margin")
    # スロットに載っている小節と、その小節の note on を予約した位置。
    occupant: dict[int, tuple[int, int]] = {}
    violations = 0
    margins: list[int] = []
    for index, group in enumerate(groups):
        slot, measure, clock = group["slot"], group["measure"], group["clock"]
        previous = occupant.get(slot)
        if previous is None:
            print(
                f"     {index:>2}  {slot:>4}  {measure:>4}  {clock:>11}"
                f"  {'-':>9}  {'-':>14}  {'-':>8}"
            )
        else:
            prev_measure, prev_at = previous
            margin = clock - prev_at
            margins.append(margin)
            if margin <= 0:
                violations += 1
            print(
                f"     {index:>2}  {slot:>4}  {measure:>4}  {clock:>11}"
                f"  {prev_measure:>9}  {prev_at:>14}  {margin:>8}"
                + ("   <-- 踏み潰し" if margin <= 0 else "")
            )
        if index < len(schedule):
            entry = schedule[index]
            if entry["measure"] != measure:
                print(
                    f"     !! 予約表とロード順が食い違っている"
                    f"（{index} 番目: ログ meas{measure} / 予約表 meas{entry['measure']}）"
                )
            occupant[slot] = (measure, entry["at_frames"])
        else:
            # 予約表に無い＝演奏が止まるまでに鳴らなかったロード。以降は判定できない。
            occupant.pop(slot, None)
    print()
    if margins and measure_frames:
        thinnest = min(margins)
        print(
            f"   余裕のいちばん薄いところ: {thinnest} frames"
            f" = {thinnest / measure_frames:.2f} 小節"
            f"（スロット {len(set(g['slot'] for g in groups))} 本を使い回している）"
        )
        print(
            "   これが「先読みがサーバーのクロックより何小節先行しても壊れないか」"
            "の実測値。スロット 2 本の頃は約 1 小節だった"
        )
    if violations:
        print(f"   NG: {violations} 件のスロットが note on の前に上書きされている")
    else:
        print("   OK: すべてのスロットが、前の小節が鳴ったあとで差し替えられている")
    return violations == 0


def read_wav(path: Path) -> tuple[np.ndarray, int]:
    """WAV を (frames, channels) の float32 で読む。32bit float 前提。"""
    data = path.read_bytes()
    if data[:4] != b"RIFF":
        raise SystemExit(f"WAV ではない: {path}")
    index = 12
    fmt = None
    payload = None
    while index + 8 <= len(data):
        chunk_id = data[index : index + 4]
        size = struct.unpack("<I", data[index + 4 : index + 8])[0]
        body = data[index + 8 : index + 8 + size]
        if chunk_id == b"fmt ":
            fmt = body
        elif chunk_id == b"data":
            payload = body
        index += 8 + size + (size & 1)
    if fmt is None or payload is None:
        raise SystemExit(f"fmt/data チャンクが無い: {path}")
    channels = struct.unpack("<H", fmt[2:4])[0]
    rate = struct.unpack("<I", fmt[4:8])[0]
    bits = struct.unpack("<H", fmt[14:16])[0]
    if bits != 32:
        raise SystemExit(f"32bit float の WAV を想定している (bits={bits}): {path}")
    values = np.frombuffer(payload, dtype="<f4")
    frames = len(values) // channels
    return values[: frames * channels].reshape(frames, channels), rate


def mono_envelope(frames: np.ndarray, window: int) -> np.ndarray:
    """位置合わせ用の粗い包絡。位相ではなく**音の出入り**で合わせる。

    live mix は master limiter を通っていて、素材そのままの振幅では出てこない。
    生波形を相関に掛けると振幅差に引きずられるので、絶対値を窓で均した包絡を使う。
    """
    mono = np.abs(frames).max(axis=1) if frames.ndim > 1 else np.abs(frames)
    usable = (len(mono) // window) * window
    if usable == 0:
        return np.zeros(0, dtype=np.float32)
    return mono[:usable].reshape(-1, window).max(axis=1).astype(np.float32)


def best_lag(reference: np.ndarray, observed: np.ndarray, max_lag: int) -> tuple[int, float]:
    """`observed` の中で `reference` が一番よく合う位置と、そのときの正規化相関。"""
    if len(reference) == 0 or len(observed) <= len(reference):
        return 0, 0.0
    reference = reference - reference.mean()
    ref_norm = float(np.linalg.norm(reference))
    if ref_norm == 0.0:
        return 0, 0.0
    best = (0, -2.0)
    for lag in range(-max_lag, max_lag + 1):
        start = max_lag + lag
        window = observed[start : start + len(reference)]
        if len(window) < len(reference):
            continue
        window = window - window.mean()
        norm = float(np.linalg.norm(window))
        if norm == 0.0:
            continue
        score = float(np.dot(reference, window)) / (ref_norm * norm)
        if score > best[1]:
            best = (lag, score)
    return best


def audible_rows(tracks: int, only_row: int) -> range:
    """録れた波形に入っている行。`--only-row` のときはその 1 行だけ。

    **鳴らす行と聴こえる行は別**（`--only-row` は mixer で他を落としているだけで、
    state load も note on も全行ぶん出ている）。期待波形を作るときに使うのは
    「聴こえる行」のほう。
    """
    if only_row:
        return range(only_row, only_row + 1)
    return range(FIRST_PLAYABLE_ROW, FIRST_PLAYABLE_ROW + tracks)


def measure_material(
    cache_dir: Path, measure: int, tracks: int, only_row: int = 0
) -> np.ndarray | None:
    """その小節の素材（聴こえる track のキャッシュ WAV の和）。**余韻まで丸ごと。**"""
    parts: list[np.ndarray] = []
    for row in audible_rows(tracks, only_row):
        path = cache_dir / f"track{row}_meas{measure}.wav"
        if not path.is_file():
            continue
        frames, _ = read_wav(path)
        parts.append(np.abs(frames).max(axis=1))
    if not parts:
        return None
    length = max(len(part) for part in parts)
    total = np.zeros(length, dtype=np.float32)
    for part in parts:
        total[: len(part)] += part
    return total


def reconstruct(
    schedule: list[dict], cache_dir: Path, tracks: int, length: int, only_row: int = 0
) -> np.ndarray:
    """**予約どおりに鳴った場合の波形**を組み立てる。

    小節ごとの素材を `at_frames` の位置へ、余韻まで含めて足し込む。キャッシュ WAV は
    小節長の約 2 倍あるので、隣り合う小節は必ず重なる。**その重なりまで入れて
    初めて、録れた波形と比べられる**（1 小節ぶんだけを参照にすると、前の小節の
    余韻のぶんだけ相関が落ちて lag の意味が無くなる）。
    """
    total = np.zeros(length, dtype=np.float32)
    cache: dict[int, np.ndarray | None] = {}
    for entry in schedule:
        measure = entry["measure"]
        if measure not in cache:
            cache[measure] = measure_material(cache_dir, measure, tracks, only_row)
        material = cache[measure]
        if material is None:
            continue
        start = entry["at_frames"]
        if start >= length:
            continue
        end = min(length, start + len(material))
        total[start:end] += material[: end - start]
    return total


def print_envelope(frames: np.ndarray, rate: int) -> None:
    """20ms ごとの音量を文字で出す。**穴（無音）が空いていれば目で見える。**"""
    window = rate // 50
    env = mono_envelope(frames, window)
    peak = float(env.max()) if len(env) else 0.0
    print("   envelope (1 文字 = 20ms, . <-60dB  - <-40  o <-20  O <-6  # >=-6)")
    line = ""
    for value in env:
        db = 20 * math.log10(value / peak) if value > 0 and peak > 0 else -99
        line += (
            "." if db < -60 else "-" if db < -40 else "o" if db < -20 else "O" if db < -6 else "#"
        )
    for i in range(0, len(line), 50):
        print(f"   {i * 20 / 1000:7.2f}s |{line[i : i + 50]}")


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


def analyse(
    path: Path,
    schedule: list[dict],
    cache_dir: Path,
    tracks: int,
    measure_frames: int,
    only_row: int = 0,
    first_clock: int = 0,
) -> bool:
    """録れた波形を測って表に出す。返すのは**刻みの合否**。

    `--only-row` を付けていないときは判定材料が無いので `True`（＝判定しない）。
    混ざった波形からは個々の打点を拾えないため。
    """
    frames, rate = read_wav(path)
    total = len(frames)
    print()
    print(f"== 録れた live mix: {path}")
    print(
        f"   frames={total} seconds={total / rate:.3f} sample_rate={rate} "
        f"measure_frames={measure_frames} ({measure_frames / rate:.4f}s)"
    )
    if total == 0:
        print("   !! 1 フレームも録れていない（live のブロックが 1 つも回っていない）")
        return False
    print(f"   peak={float(np.abs(frames).max()):.4f}")
    print_envelope(frames, rate)

    if not schedule:
        print("   (小節の予約表が無いので位置合わせは省略)")
        return False

    # 包絡どうしで合わせる。1 窓 = 1ms あれば、モタりとして聴こえる量（数 ms 以上）は
    # 充分に見える。
    window = rate // 1000
    observed = mono_envelope(frames, window)
    expected = mono_envelope(
        reconstruct(schedule, cache_dir, tracks, total, only_row).reshape(-1, 1), window
    )
    max_lag = SEARCH_FRAMES // window

    print()
    print("   小節ごとの照合（lag が実測のずれ。0 なら予約どおりのサンプルで鳴っている）")
    print("     meas  at_frames   lag_frames    lag_ms   corr")
    lags: list[int] = []
    for entry in schedule:
        # 参照は「予約どおりに鳴ったはずの波形」から、その小節ぶんを切り出す。
        head = (entry["at_frames"] - first_clock) // window
        span = measure_frames // window
        reference = expected[head : head + span]
        start = head - max_lag
        if start < 0 or len(reference) < span or head + span + max_lag > len(observed):
            continue
        segment = observed[start : head + span + max_lag]
        lag_windows, corr = best_lag(reference, segment, max_lag)
        lag_frames = lag_windows * window
        lags.append(lag_frames)
        print(
            f"     {entry['measure']:>4}  {entry['at_frames']:>9}  {lag_frames:>11}"
            f"  {lag_frames / rate * 1000:>8.1f}  {corr:>5.2f}"
        )

    if lags:
        print()
        print(
            f"   lag_frames min={min(lags)} max={max(lags)} spread={max(lags) - min(lags)}"
            f" ({(max(lags) - min(lags)) / rate * 1000:.1f}ms)"
        )

    if not only_row:
        return True

    # ここから先は 1 行だけ録ったときにしか読めない。混ざった波形では打点が重なって
    # アタックを 1 つずつ拾えず、余韻も他の行の音と見分けが付かない。
    def material_of(measure: int) -> np.ndarray | None:
        return measure_material(cache_dir, measure, tracks, only_row)

    attacks_ok = daw_live_attacks.report(
        frames, rate, schedule, material_of, measure_frames, first_clock
    )
    daw_live_material_match.report(
        frames,
        rate,
        schedule,
        material_of,
        measure_frames,
        first_clock,
        # 照合表で測った固定のずれ。これを外して比べると、全体がずれているだけで
        # 「切れている」と誤って数えてしまう。
        int(np.median(lags)) if lags else 0,
    )
    return attacks_ok


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
