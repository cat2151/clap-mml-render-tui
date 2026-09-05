#!/usr/bin/env python3
"""録れた live mix から**アタックの位置を 1 つずつ**拾って、刻みの崩れを数える。

`capture_daw_live_mix.py` から使う。単体では動かさない。

## なぜ相互相関では足りないのか

`capture_daw_live_mix.py` の照合表は「小節まるごと」を 1 つの塊として合わせる。
それで「別の小節が鳴っている」（資料の事実 2）は捕まえられたが、**小節の中の
どこがどう崩れているか**は 1 つの `lag` へ潰れてしまう。7 本が混ざった波形では
個々のアタックが重なるので、それ以上は分けられない。

hi-hat は **鋭いアタックが等間隔に並ぶ**ので、1 本だけ録れば話が変わる。
アタックを 1 つずつ拾って並べれば、

- 小節の中の刻みは合っているのに、**小節の切り替わりだけ間隔が違う**
- それとも小節まるごとが前後にずれている

のどちらなのかが、聴かなくても数で分かれる。

## 測り方

期待側は**素材そのもの**から作る。その小節のキャッシュ WAV のアタック位置を拾い、
予約位置（`at_frames`）を足したものが「そこで鳴るはずだった位置」。録れた波形から
拾ったアタックと 1 つずつ突き合わせる。

同じ小節の中のアタックは 1 本の WAV の中に焼かれているので、**再生開始が動かない限り
互いの間隔は 1 サンプルも変わらない。** つまり小節内のずれが揃っているかどうかを見れば、
「小節の頭の置き方の問題」と「素材の中身の問題」が切り分けられる。
"""

from __future__ import annotations

import numpy as np

# アタックとみなす閾値（その波形のピークに対する比）。hi-hat は減衰が速いので、
# 4 分の 1 まで落ちれば次の打点と混ざらない。
THRESHOLD_RATIO = 0.25
# いったんアタックを取ったあと、次を取れるようになる水準（閾値に対する比）。
RELEASE_RATIO = 0.4
# 包絡の窓（フレーム）。ここは「アタックを見つける」ためだけの粗さで、位置そのものは
# 見つけた窓の中をサンプル単位で走査して決める。
ENVELOPE_WINDOW = 64
# 打点のずれをここまでは許す（フレーム）。1 オーディオブロック = 512 フレーム
# （10.7ms）に収まっていれば聴こえない、という資料の基準に合わせてある。
# アタックの鈍い行（ベースなど）は検出そのものが ±115 ほどぶれるので、
# これより厳しくすると検出の誤差で赤くなる。
TOLERANCE_FRAMES = 512
# 期待した打点のうち、これだけ対応が付かないと判定そのものを信用しない。
# 録音の終わりで切れたぶんは対応が付かないので、少し緩めにしてある。
MIN_MATCH_RATIO = 0.6


def _mono(frames: np.ndarray) -> np.ndarray:
    """チャンネルをまたいだ絶対値の最大。位相ではなく音の出入りだけを見る。"""
    if frames.ndim == 1:
        return np.abs(frames)
    return np.abs(frames).max(axis=1)


def detect_attacks(frames: np.ndarray, threshold_ratio: float = THRESHOLD_RATIO) -> list[int]:
    """アタックの位置（フレーム）を先頭から並べる。

    粗い包絡で「どのあたりか」を決め、**その窓の中をサンプル単位で走査して**
    立ち上がりの先頭を返す。窓の粗さ（64 フレーム = 1.3ms）がそのまま誤差に
    ならないようにするため。
    """
    mono = _mono(frames)
    usable = (len(mono) // ENVELOPE_WINDOW) * ENVELOPE_WINDOW
    if usable == 0:
        return []
    envelope = mono[:usable].reshape(-1, ENVELOPE_WINDOW).max(axis=1)
    peak = float(envelope.max())
    if peak <= 0.0:
        return []
    threshold = peak * threshold_ratio
    attacks: list[int] = []
    armed = True
    for index, value in enumerate(envelope):
        if armed and value >= threshold:
            armed = False
            # 立ち上がりは窓の途中から始まっている。1 つ前の窓の頭から走査して、
            # 閾値の半分を最初に超えたサンプルをアタックの位置とする。
            start = max(0, (index - 1) * ENVELOPE_WINDOW)
            stop = (index + 1) * ENVELOPE_WINDOW
            local = np.nonzero(mono[start:stop] >= threshold * 0.5)[0]
            attacks.append(start + int(local[0]) if len(local) else index * ENVELOPE_WINDOW)
        elif not armed and value < threshold * RELEASE_RATIO:
            armed = True
    return attacks


def expected_attacks(
    schedule: list[dict],
    material_of: callable,
    measure_frames: int,
    first_clock: int,
) -> list[dict]:
    """「予約どおりなら、ここで鳴るはず」というアタックを全部並べる。

    素材のアタック位置は小節の中のオフセット。**小節長より後ろは切り捨てる**
    （キャッシュ WAV は余韻ぶん小節より長く、次の小節と重なる領域には次の小節の
    素材が乗るので、そこを期待値に混ぜると二重に数えてしまう）。

    `first_clock` は録音の原点にあたるサンプルクロック。`at_frames` は
    クロックの絶対位置なので、WAV の中の位置へ直すにはこれを引く。
    """
    result: list[dict] = []
    cache: dict[int, list[int]] = {}
    for play, entry in enumerate(schedule):
        measure = entry["measure"]
        if measure not in cache:
            material = material_of(measure)
            cache[measure] = (
                []
                if material is None
                else [a for a in detect_attacks(material) if a < measure_frames]
            )
        base = entry["at_frames"] - first_clock
        for index, offset in enumerate(cache[measure]):
            result.append(
                {
                    # 同じ小節がループで何度も鳴るので、**何周目か**（`play`）を
                    # 持たせないと小節ごとの立ち位置がまとめられない。
                    "play": play,
                    "measure": measure,
                    "index": index,
                    "at": base + offset,
                    "offset": offset,
                    "first_of_measure": index == 0,
                }
            )
    result.sort(key=lambda item: item["at"])
    return result


def match_attacks(expected: list[dict], observed: list[int], tolerance: int) -> list[dict]:
    """期待した打点ごとに、いちばん近い実測の打点を割り当てる。

    見つからなければ `observed=None`（＝その打点は鳴らなかった、または閾値に
    届かなかった）。**取りこぼしをそのまま出す**のが要点で、黙って詰めると
    以降の間隔が全部ずれて読めなくなる。
    """
    matched: list[dict] = []
    array = np.array(observed, dtype=np.int64)
    for item in expected:
        row = dict(item)
        row["observed"] = None
        row["delta"] = None
        if len(array):
            nearest = int(np.argmin(np.abs(array - item["at"])))
            if abs(int(array[nearest]) - item["at"]) <= tolerance:
                row["observed"] = int(array[nearest])
                row["delta"] = row["observed"] - item["at"]
        matched.append(row)
    return matched


def _stats(values: list[int]) -> str:
    if not values:
        return "（無し）"
    array = np.array(values)
    return (
        f"n={len(array)} min={array.min()} max={array.max()} "
        f"spread={array.max() - array.min()} mean={array.mean():.1f}"
    )


def report(
    frames: np.ndarray,
    rate: int,
    schedule: list[dict],
    material_of: callable,
    measure_frames: int,
    first_clock: int,
) -> bool:
    """アタックの一覧・間隔・小節ごとのずれを表にして出し、合否を返す。

    合格の条件は 2 つだけ。どちらも**素材と突き合わせて初めて言えること**で、
    予約表（`at_frames`）の間隔をいくら測っても出てこない:

    - **1 つの小節の中で、打点のずれが揃っていること**（`drift_spread`）。
      1 小節は 1 本の WAV なので、頭が正しい位置で鳴り始めたなら残りも
      素材のとおりに並ぶはず。揃っていないなら、**その小節の再生の途中で
      音源が飛んでいる**（実測ではここが 2560 フレーム = 5 ブロック飛んでいた）
    - **小節をまたぐ打点の間隔が素材どおりであること**（`gap_diff`）。
      小節の頭だけが前後していれば、ここに出る
    """
    observed = detect_attacks(frames)
    print()
    print("== アタックの一覧（1 行だけ録ったときに読める。混ざった波形では意味が無い）")
    print(f"   録れた波形のアタック: {len(observed)} 個  first_clock={first_clock}")
    if not schedule:
        print("   (予約表が無いので突き合わせは省略)")
        return False
    expected = expected_attacks(schedule, material_of, measure_frames, first_clock)
    if not expected:
        print("   !! 素材からアタックを 1 つも拾えなかった（この行は打楽器ではない可能性）")
        return False
    spacing = int(np.median(np.diff([item["at"] for item in expected]))) if len(expected) > 1 else measure_frames
    tolerance = max(1, spacing // 2)
    matched = match_attacks(expected, observed, tolerance)

    deltas = [row["delta"] for row in matched if row["delta"] is not None]
    if not deltas:
        print("   !! 期待した位置の近くに実測の打点が 1 つも無い（別の音が鳴っている）")
        return False
    baseline = int(np.median(deltas))
    print(
        f"   期待した打点: {len(expected)} 個  対応が付いた: {len(deltas)} 個  "
        f"素材の打点間隔（中央値）={spacing} frames ({spacing / rate * 1000:.1f}ms)"
    )
    print(
        f"   ずれの中央値 = {baseline} frames ({baseline / rate * 1000:.1f}ms)。"
        "下の drift はここを 0 に置き直したもの（＝**打点ごとの崩れだけ**を見る）"
    )
    print()
    print("     meas   #   expected   observed   delta   drift    gap_obs   gap_exp   gap_diff")
    inside: list[int] = []
    across: list[int] = []
    previous: dict | None = None
    for row in matched:
        gap_obs = gap_exp = gap_diff = None
        if previous is not None and previous["observed"] is not None and row["observed"] is not None:
            gap_obs = row["observed"] - previous["observed"]
            gap_exp = row["at"] - previous["at"]
            gap_diff = gap_obs - gap_exp
            (across if row["first_of_measure"] else inside).append(gap_diff)
        mark = " <-- 小節の切り替わり" if row["first_of_measure"] else ""
        print(
            f"     {row['measure']:>4}  {row['index']:>2}  {row['at']:>9}"
            f"  {'-' if row['observed'] is None else row['observed']:>9}"
            f"  {'-' if row['delta'] is None else row['delta']:>6}"
            f"  {'-' if row['delta'] is None else row['delta'] - baseline:>6}"
            f"  {'-' if gap_obs is None else gap_obs:>9}"
            f"  {'-' if gap_exp is None else gap_exp:>9}"
            f"  {'-' if gap_diff is None else gap_diff:>9}" + mark
        )
        previous = row

    print()
    print("   打点の間隔が素材どおりか（gap_diff。0 なら素材のとおりに刻めている）")
    print(f"     小節の中    : {_stats(inside)}")
    print(f"     小節をまたぐ: {_stats(across)}")
    print()
    print("   小節ごとの立ち位置（drift。同じ小節の中は 1 本の WAV なので必ず揃うはず）")
    print("     周   meas   打点   drift_mean   drift_spread")
    spreads: list[int] = []
    for play, entry in enumerate(schedule):
        rows = [
            row["delta"] - baseline
            for row in matched
            if row["play"] == play and row["delta"] is not None
        ]
        if not rows:
            continue
        array = np.array(rows)
        print(
            f"     {play:>2}   {entry['measure']:>4}   {len(array):>4}   {array.mean():>10.1f}"
            f"   {array.max() - array.min():>12}"
            + ("   <-- 小節の途中で音源が飛んでいる" if array.max() - array.min() > TOLERANCE_FRAMES else "")
        )
        spreads.append(int(array.max() - array.min()))

    return _verdict(spreads, across, len(deltas), len(expected), rate)


def _verdict(
    spreads: list[int], across: list[int], matched: int, expected: int, rate: int
) -> bool:
    """表に出した数値から合否を決める。**耳を使わずにここで白黒が付く。**"""
    print()
    print("== 刻みの判定")
    if expected == 0 or matched < expected * MIN_MATCH_RATIO:
        print(
            f"   NG: 期待した打点 {expected} 個のうち {matched} 個しか対応が付かない"
            "（別の音が鳴っているか、閾値が合っていない）"
        )
        return False
    worst_inside = max(spreads) if spreads else 0
    worst_across = max((abs(v) for v in across), default=0)
    ok = worst_inside <= TOLERANCE_FRAMES and worst_across <= TOLERANCE_FRAMES
    print(
        f"   小節の中のずれの揃い（最大 drift_spread）= {worst_inside} frames"
        f" ({worst_inside / rate * 1000:.1f}ms)"
    )
    print(
        f"   小節をまたぐ間隔のずれ（最大 |gap_diff|）= {worst_across} frames"
        f" ({worst_across / rate * 1000:.1f}ms)"
    )
    if ok:
        print(f"   OK: どちらも許容 {TOLERANCE_FRAMES} frames（1 オーディオブロック）以内")
    else:
        print(
            f"   NG: 許容 {TOLERANCE_FRAMES} frames（1 オーディオブロック = 10.7ms）を超えている。"
            "小節の中が揃っていないなら再生の途中で音源が飛んでいる"
        )
    return ok
