#!/usr/bin/env python3
"""録れた 1 行が**素材そのものか**を、サンプル単位の残差で見る。

`capture_daw_live_mix.py` から `--only-row` のときだけ使う。単体では動かさない。

## アタックの一覧だけでは足りないもの

`daw_live_attacks.py` は**打点の位置**しか見ない。hi-hat のように減衰が速い行なら
それで足りるが、パッドやベースのように**小節をまたいで伸びる行**では、位置が合って
いても

- 前の小節の余韻が、次の小節の頭で**切られている**（スロットの差し替えで音源が
  取り上げられた）
- 音量が素材と違う（limiter に当たった、gain が二重に掛かった）

といった崩れが残る。**「ぶつ切り」はここにしか出ない。**

## 測り方

素材を予約位置へ並べ直したものと、録れた波形を、20ms の窓ごとに比べる。
振幅の比（mixer の gain ぶん）は最小二乗で 1 つだけ求めて全体へ当てる。

- `level` … その窓で素材が期待する音量（フルスケール比）
- `ratio` … 実測 ÷ 期待。**1.0 なら素材どおり**。0 に近ければ音が消えている
- 「切れた」と数えるのは、**期待が聴こえる大きさ（-40dB 以上）なのに実測が
  その 1/4 未満**の窓。減衰の読み違いを拾わないよう、素材が鳴っていない窓は数えない
"""

from __future__ import annotations

import numpy as np

# 窓（ms）。これより細かくすると位相のずれを崩れと読み違える。
WINDOW_MS = 20
# 「聴こえる大きさ」の下限（期待側のフルスケール比）。-40dB。
AUDIBLE = 0.01
# 実測がこの比を下回ったら「切れている」と数える。
CUT_RATIO = 0.25


def _envelope(mono: np.ndarray, window: int) -> np.ndarray:
    usable = (len(mono) // window) * window
    if usable == 0:
        return np.zeros(0, dtype=np.float32)
    return mono[:usable].reshape(-1, window).max(axis=1)


def report(
    frames: np.ndarray,
    rate: int,
    schedule: list[dict],
    material_of: callable,
    measure_frames: int,
    first_clock: int,
    lag: int,
) -> None:
    """素材との残差を小節ごとに出す。`lag` は照合表で測った固定のずれ（フレーム）。"""
    observed = np.abs(frames).max(axis=1) if frames.ndim > 1 else np.abs(frames)
    length = len(observed)
    expected = np.zeros(length, dtype=np.float32)
    cache: dict[int, np.ndarray | None] = {}
    for entry in schedule:
        measure = entry["measure"]
        if measure not in cache:
            cache[measure] = material_of(measure)
        material = cache[measure]
        if material is None:
            continue
        start = entry["at_frames"] - first_clock + lag
        if start >= length:
            continue
        head = max(0, start)
        end = min(length, start + len(material))
        if end <= head:
            continue
        expected[head:end] += material[head - start : end - start]

    window = rate * WINDOW_MS // 1000
    observed_env = _envelope(observed, window)
    expected_env = _envelope(expected, window)
    span = min(len(observed_env), len(expected_env))
    observed_env, expected_env = observed_env[:span], expected_env[:span]
    loud = expected_env >= AUDIBLE
    print()
    print("== 素材との突き合わせ（余韻が切られていないか。1 行だけ録ったときに読める）")
    if not loud.any():
        print("   素材が聴こえる大きさの窓が無い（この行は鳴っていない）")
        return
    # 振幅比は 1 つだけ。行ごとに mixer の gain が掛かっているぶんを吸収する。
    scale = float(np.dot(observed_env[loud], expected_env[loud]) / np.dot(expected_env[loud], expected_env[loud]))
    ratio = np.zeros(span, dtype=np.float32)
    np.divide(observed_env, expected_env * scale, out=ratio, where=loud)
    cut = loud & (ratio < CUT_RATIO)
    print(
        f"   振幅比 scale={scale:.4f} ({20 * np.log10(max(scale, 1e-9)):.1f}dB)"
        f"  聴こえる窓={int(loud.sum())}  切れている窓={int(cut.sum())}"
        f"  (窓 {WINDOW_MS}ms、期待 -40dB 以上で実測がその {CUT_RATIO:.0%} 未満)"
    )
    print()
    print("     周   meas    聴こえる窓   切れている窓   最小 ratio   小節の頭からの位置")
    for play, entry in enumerate(schedule):
        head = (entry["at_frames"] - first_clock + lag) // window
        tail = head + measure_frames // window
        if head < 0 or head >= span:
            continue
        tail = min(tail, span)
        block_loud = loud[head:tail]
        if not block_loud.any():
            continue
        block_ratio = ratio[head:tail]
        block_cut = cut[head:tail]
        worst = int(np.argmin(np.where(block_loud, block_ratio, 2.0)))
        print(
            f"     {play:>2}   {entry['measure']:>4}   {int(block_loud.sum()):>9}"
            f"   {int(block_cut.sum()):>12}   {float(block_ratio[worst]):>10.2f}"
            f"   {worst * WINDOW_MS:>10}ms"
        )
    if int(cut.sum()) == 0:
        print()
        print("   OK: 素材が鳴っているところは全部、素材どおりの音量で出ている（切れていない）")
