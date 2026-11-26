#!/usr/bin/env python3
"""DAW のキャッシュ WAV に「何世代のテンポが同居しているか」を、ファイルだけで判定する。

    python scripts/check_daw_cache_staleness.py
    python scripts/check_daw_cache_staleness.py --dir <キャッシュのディレクトリ>
    python scripts/check_daw_cache_staleness.py --json out.json
    python scripts/check_daw_cache_staleness.py --log auto      # 実行ログとの突き合わせ

判定の背景は docs/adr/0018-page-replacement-clears-the-cache.md。

## なぜファイルの長さだけで分かるか

キャッシュ WAV は「1 小節ぶん＋余韻の尾」を焼いたもの。小節の長さは BPM と拍子で
決まるので、**同じ project から焼かれた WAV はすべて同じフレーム数になる。**
長さが 2 種類以上あれば、それは 2 つ以上の世代（＝別の日の別の BPM）の WAV が
同じディレクトリに同居しているということで、**耳も実サーバーも要らずに黒が出る。**

判定の芯は **「同じディレクトリの中で長さが 2 種類以上あれば NG」** で、
尾の長さには依存しない。BPM の逆算は**説明のための表示**（`--tail-frames` で
仮定する尾を変えられる）で、既定では判定に入れない。renderer が余韻の長さを
変えた瞬間に全部赤くなる形の判定を主にしないため。`--strict-tail` を付けたときだけ
「今日の小節長＋仮定した尾」との一致も判定に入れる。

## project との突き合わせ

`daily_daw/current.json` を読んで、**今日の project に 1 文字も無いのに WAV が
在る行**を出す（資料の事実 4 の機械化）。保存ファイルの track 番号とグリッドの
行 index はずれる（保存 `track: 1` = グリッド行 2 = `track2_*.wav`）ので、
ここで変換している。**`non_empty_cells` が空でも、init セルに
`generate from chord track` があって chord 行に中身があれば「鳴る行」**である点も
再現してある（`daw/src/mml.rs` の `cell_has_content`）。

## 実行ログとの突き合わせ（`--log`）

ファイルだけを見ても「実アプリが本当にその古い WAV を鳴らしたか」は分からない。
`--log` を付けると、`log/log.txt` の最後のセッションから
`meas<N>: live-cache ... sent=row2/i0,...` を拾い、**鳴らした行の集合と project に
中身のある行の集合を引き算して差を数で出す**（`daw_log_sent_rows.py`）。
差が 1 行でもあれば exit 1。

## 出口

- exit 0 … 世代の混在なし・project に無い行の WAV もなし・ログの sent= も project どおり
- exit 1 … どれかが NG（何が NG かは表に出る）

**`%LOCALAPPDATA%` は読むだけ。1 バイトも書き換えない。** 掃除は直し（Stage 7）の
仕事で、しかもユーザーの判断が要る。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

import daw_log_sent_rows
from daw_cache_wav import WavError, read_wav_info
from daw_project import FIRST_PLAYABLE_TRACK, FIRST_SAVED_PLAYABLE_TRACK, Project, load_project

APP_DIR_NAME = "clap-mml-render-tui"
DAW_CACHE_DIR_NAME = "daw_cache"
DAILY_DIR_NAME = "daily"
# キャッシュ WAV のファイル名。track はグリッドの行 index、meas は 1 始まり。
CACHE_WAV_RE = re.compile(r"^track(\d+)_meas(\d+)\.wav$", re.IGNORECASE)
# 余韻の尾の既定値（フレーム）。実測 2.0 秒 @48kHz。**判定には既定で使わない**。
DEFAULT_TAIL_FRAMES = 96_000
# 実測のフレーム数は丸めで ±1 ずれうる。
FRAME_TOLERANCE = 1


# ─── パス解決 ────────────────────────────────────────────────


def local_app_dir() -> Path:
    """`%LOCALAPPDATA%\\clap-mml-render-tui`。環境変数が無ければ HOME から組み立てる。"""
    base = os.environ.get("LOCALAPPDATA")
    if base:
        return Path(base) / APP_DIR_NAME
    return Path.home() / "AppData" / "Local" / APP_DIR_NAME


def default_daily_dirs() -> list[Path]:
    """`daw_cache/<plugin>/daily` を全部返す。

    **プラグイン名前空間を直書きしないこと**（`core-lib/src/cache_dirs.rs`）。
    既定プラグインで決まるので `Surge XT` とは限らず、テストでは `unknown-plugin`。
    """
    root = local_app_dir() / DAW_CACHE_DIR_NAME
    if not root.is_dir():
        return []
    return sorted(p for p in root.glob(f"*/{DAILY_DIR_NAME}") if p.is_dir())


def default_project_path() -> Path:
    return local_app_dir() / "daily_daw" / "current.json"


def default_log_path() -> Path:
    """実アプリのデバッグログ（`cmrt_runtime::paths::log_file_path` と同じ場所）。"""
    return local_app_dir() / "log" / "log.txt"



# ─── 走査 ────────────────────────────────────────────────────


def collect_wavs(directory: Path) -> list[dict]:
    entries = []
    for path in sorted(directory.iterdir()):
        if not path.is_file():
            continue
        match = CACHE_WAV_RE.match(path.name)
        if not match:
            continue
        entry = {
            "path": path,
            "name": path.name,
            "row": int(match.group(1)),
            "measure": int(match.group(2)),
        }
        try:
            entry.update(read_wav_info(path))
        except (WavError, OSError) as err:
            entry["error"] = str(err)
        entries.append(entry)
    return entries


def compact_names(entries: list[dict], limit: int = 2) -> str:
    names = [entry["name"] for entry in entries]
    if len(names) <= limit:
        return ", ".join(names)
    return f"{names[0]} .. {names[-1]} ({len(names)} files)"


def bpm_from_frames(frames: int, tail: int, beat: int, sample_rate: int) -> float | None:
    body = frames - tail
    if body <= 0:
        return None
    return beat * 60.0 * sample_rate / body


def check_directory(
    directory: Path,
    project: Project | None,
    rows_check: bool,
    tail_frames: int,
    strict_tail: bool,
    sample_rate: int,
) -> tuple[bool, dict]:
    """1 ディレクトリぶんの判定。戻り値は (OK か, JSON 用の dict)。"""
    print()
    print(f"== キャッシュ WAV の世代: {directory}")
    entries = collect_wavs(directory)
    result: dict = {"dir": str(directory), "files": len(entries), "problems": []}
    if not entries:
        print("   （キャッシュ WAV が 1 つも無い）")
        result["generations"] = []
        return True, result

    broken = [entry for entry in entries if "error" in entry]
    for entry in broken:
        print(f"   !! 読めない: {entry['name']}: {entry['error']}   NG")
    if broken:
        result["problems"].append("unreadable-wav")

    good = [entry for entry in entries if "error" not in entry]
    expected_measure = project.measure_frames(sample_rate) if project else None
    beat = project.beat_numerator() if project else 4
    if project:
        print(
            f"   今日の project: BPM {project.bpm():g} / beat {beat} 拍 "
            f"-> 小節 {expected_measure} frames"
            f"（尾 {tail_frames} を仮定すると WAV は {expected_measure + tail_frames} frames）"
        )

    groups: dict[int, list[dict]] = defaultdict(list)
    for entry in good:
        groups[entry["frames"]].append(entry)
    generations = []
    for frames in sorted(groups, reverse=True):
        members = groups[frames]
        bpm = bpm_from_frames(frames, tail_frames, beat, sample_rate)
        bpm_text = f"BPM {bpm:.1f}" if bpm else "BPM 不明"
        matches_today = bool(
            expected_measure is not None
            and abs(frames - (expected_measure + tail_frames)) <= FRAME_TOLERANCE
        )
        if expected_measure is None:
            verdict = ""
        elif matches_today:
            verdict = "OK（今日の長さ）"
        else:
            verdict = "NG（今日の長さではない）" if strict_tail else "?（今日の長さではない）"
        print(
            f"   {frames:>8} frames  x{len(members):<3} "
            f"{compact_names(members):<40} -> {bpm_text:<10} {verdict}"
        )
        generations.append(
            {
                "frames": frames,
                "count": len(members),
                "inferred_bpm": round(bpm, 3) if bpm else None,
                "matches_today": matches_today,
                "files": [entry["name"] for entry in members],
            }
        )
    result["generations"] = generations

    ok = not broken
    if len(groups) > 1:
        print(f"   !! 長さが {len(groups)} 種類ある（1 種類なら OK）= 世代の混在   NG")
        result["problems"].append("mixed-generations")
        ok = False
    elif groups:
        print("   長さは 1 種類。世代の混在なし   OK")

    if strict_tail and expected_measure is not None:
        stale = [gen for gen in generations if not gen["matches_today"]]
        if stale:
            print(f"   !! 今日の小節長＋尾 {tail_frames} に一致しない長さが {len(stale)} 種類   NG")
            result["problems"].append("not-todays-length")
            ok = False

    formats = {(entry["channels"], entry["sample_rate"], entry["bits"]) for entry in good}
    if len(formats) > 1:
        print(f"   !! フォーマットが揃っていない: {sorted(formats)}   NG")
        result["problems"].append("mixed-formats")
        ok = False

    if rows_check and project is not None:
        ok = check_rows(entries, project, result) and ok
    return ok, result


def check_rows(entries: list[dict], project: Project, result: dict) -> bool:
    """「今日の project に無いのに WAV が在る」セルを出す（資料の事実 4）。"""
    print("== project に無いのに WAV が在る行")
    print(f"   project: {project.path}  page_date={project.page_date}")
    rows_with_wav = sorted({entry["row"] for entry in entries})
    content_rows = project.rows_with_content()
    orphan_rows = []
    orphan_cells = []
    for row in rows_with_wav:
        members = [entry for entry in entries if entry["row"] == row]
        saved_track = row - (FIRST_PLAYABLE_TRACK - FIRST_SAVED_PLAYABLE_TRACK)
        if row not in content_rows:
            orphan_rows.append(row)
            print(
                f"   row{row} (保存 track {saved_track}): project に 1 文字も無いのに "
                f"{len(members)} files: {compact_names(members)}   NG"
            )
            continue
        missing = [
            entry for entry in members if not project.cell_has_content(row, entry["measure"])
        ]
        if missing:
            orphan_cells.extend(entry["name"] for entry in missing)
            print(
                f"   row{row} (保存 track {saved_track}): 行は在るが "
                f"{len(missing)} 小節ぶんが project に無い: {compact_names(missing)}   NG"
            )
        else:
            print(f"   row{row} (保存 track {saved_track}): {len(members)} files   OK")
    result["rows_with_wav"] = rows_with_wav
    result["rows_with_content"] = sorted(content_rows)
    result["orphan_rows"] = orphan_rows
    result["orphan_cells"] = orphan_cells
    if orphan_rows or orphan_cells:
        result["problems"].append("wav-without-project-content")
        return False
    print("   project に無い行の WAV は無し   OK")
    return True


def resolve_rows_check(mode: str, directory: Path) -> bool:
    if mode == "on":
        return True
    if mode == "off":
        return False
    return directory.name.lower() == DAILY_DIR_NAME


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--dir",
        dest="dirs",
        action="append",
        type=Path,
        help="調べるキャッシュのディレクトリ（複数可）。既定は daw_cache/*/daily を全部",
    )
    parser.add_argument(
        "--project",
        default="auto",
        help="current.json のパス。'none' で project との突き合わせを切る（既定 auto）",
    )
    parser.add_argument(
        "--rows-check",
        choices=["auto", "on", "off"],
        default="auto",
        help="project に無い行の検出。auto はディレクトリ名が daily のときだけ on",
    )
    parser.add_argument(
        "--tail-frames",
        type=int,
        default=DEFAULT_TAIL_FRAMES,
        help=f"余韻の尾のフレーム数（BPM 逆算の表示に使う。既定 {DEFAULT_TAIL_FRAMES}）",
    )
    parser.add_argument(
        "--strict-tail",
        action="store_true",
        help="「今日の小節長＋尾」と一致しない長さも NG にする（既定は表示だけ）",
    )
    parser.add_argument(
        "--log",
        help="実アプリのログを読んで sent= と project を突き合わせる。'auto' で既定のログ",
    )
    parser.add_argument(
        "--log-session",
        type=int,
        default=-1,
        help="何番目の '=== DAW mode ready ===' から見るか。負数は最後から（既定 -1）",
    )
    parser.add_argument("--sample-rate", type=int, default=48_000)
    parser.add_argument("--json", type=Path, help="判定を JSON でも書き出す")
    args = parser.parse_args()

    directories = args.dirs or default_daily_dirs()
    if not directories:
        print("調べるディレクトリが無い（--dir で指定すること）", file=sys.stderr)
        return 1

    project = None
    if args.project != "none":
        project_path = default_project_path() if args.project == "auto" else Path(args.project)
        if project_path.is_file():
            project = load_project(project_path)
        elif args.project != "auto":
            print(f"project が無い: {project_path}", file=sys.stderr)
            return 1
        else:
            print(f"（project が無いので突き合わせは省略: {project_path}）")

    all_ok = True
    results = []
    for directory in directories:
        if not directory.is_dir():
            print(f"ディレクトリが無い: {directory}", file=sys.stderr)
            all_ok = False
            continue
        ok, result = check_directory(
            directory,
            project,
            resolve_rows_check(args.rows_check, directory),
            args.tail_frames,
            args.strict_tail,
            args.sample_rate,
        )
        results.append(result)
        all_ok = ok and all_ok

    log_result: dict | None = None
    if args.log:
        log_path = default_log_path() if args.log == "auto" else Path(args.log)
        if not log_path.is_file():
            print(f"ログが無い: {log_path}", file=sys.stderr)
            return 1
        log_result = {}
        try:
            log_ok = daw_log_sent_rows.check_log(log_path, args.log_session, project, log_result)
        except ValueError as err:
            print(f"ログが読めない: {err}", file=sys.stderr)
            return 1
        all_ok = log_ok and all_ok

    print()
    verdict_ok = "OK（世代の混在なし" + ("・ログの sent= も project どおり）" if args.log else "）")
    print("判定: " + (verdict_ok if all_ok else "NG（上の表の NG を見ること）"))
    if args.json:
        args.json.write_text(
            json.dumps(
                {"ok": all_ok, "directories": results, "log": log_result},
                ensure_ascii=False,
                indent=2,
            ),
            encoding="utf-8",
        )
        print(f"JSON: {args.json}")
    return 0 if all_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
