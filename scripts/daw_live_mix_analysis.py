"""録れた live mix を、予約表から再構成した期待波形と突き合わせる（`capture_daw_live_mix.py` 用）。"""

from __future__ import annotations

import math
import struct
from pathlib import Path

import numpy as np

import daw_live_attacks
import daw_live_material_match

FIRST_PLAYABLE_ROW = 2
# 相互相関で探す範囲（フレーム）。小節長の 1/4 も探せば充分で、これ以上広げると
# 隣の小節のピークを拾ってしまう。
SEARCH_FRAMES = 12_000


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
