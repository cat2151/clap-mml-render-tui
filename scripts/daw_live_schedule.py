"""演奏ループの小節ログとサーバーログから、予約表・原点・スロット上書きを判定する（`capture_daw_live_mix.py` 用）。"""

from __future__ import annotations

import re
import sys
from pathlib import Path

SAMPLE_RATE = 48_000


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
