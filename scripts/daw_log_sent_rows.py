#!/usr/bin/env python3
"""実アプリのログから「どの行を鳴らしたか」を取り出して、project と突き合わせる。

`check_daw_cache_staleness.py --log <path>` の実体（入口はあちら側だけ）:

    python scripts/check_daw_cache_staleness.py --log auto
    python scripts/check_daw_cache_staleness.py --log <path> --log-session -2

判定の背景は docs/adr/0018-page-replacement-clears-the-cache.md。

## なぜ要るか

テストとスクリプトは、**実アプリが本当にその経路を通っている証拠ではない。** この repo の過去の調査では、静的な読みと使い捨ての example が
実在しない誤分類をでっち上げた。だから最後は**動いたアプリが吐いたログ**を見る。

ただし「ログを目で読む」で終わらせない。ログの `sent=` に並ぶ行と、`current.json`
の project に中身のある行を**集合として引き算して、差を数で出す。**
差が 1 行でもあれば、それは「project に 1 文字も無い行が鳴っている」ということで、
そのまま exit 1 になる。

## セッションの切り出し

ログは 200MB を超える追記ファイルで、何十回ぶんもの起動が入っている。
`=== DAW mode ready ===` を起点として区切り、既定では**最後のセッション**だけを見る。
`--session` に負数を渡すと「最後から N 番目」。

## 読む 4 行（資料の Stage 4 の手順そのまま）

- `daily rollover: <前日> -> <当日>; archive=...`（Resume の日は出ない）
- `daily cache cleared: dir=...; removed=<n> wav`（rollover 成功時の掃除。**rollover が
  出ているのにこれが無ければ NG**）
- `Grid history を全置換import: ...` と `grid import cache cleared: dir=...; removed=<n> wav`
  （日中の全置換とその掃除。**import が出ているのに掃除の行が無ければ NG**。
  rollover とは別の入口で、同じ陳腐化を日中に作る）
- `offline-render: ... request_id=<n> ... mml_hash=<h>`（今日焼き直した件数）
- `realtime-play: action=shm-patch-prepare event=start ... track<N>_meas<M>.wav`
- `meas<N>: live-cache ... sent=row2/i0,... silent=row9 ...`

**行番号はグリッドの行 index** で、ログの `sent=row2/i0` と `track2_*.wav` の 2 は
同じ体系。保存ファイルの track 番号（画面の T1）とは 1 ずれるので混ぜないこと。
"""

from __future__ import annotations

import re
from collections import OrderedDict
from pathlib import Path

# `=== DAW mode ready ===` がセッションの起点。時刻の prefix ごと拾う。
SESSION_RE = re.compile(r"^\[(?P<ts>[^\]]*)\]\s*=== DAW mode ready ===")
ROLLOVER_RE = re.compile(r"daily rollover: (?P<from>\S+) -> (?P<to>[^;]+); archive=(?P<archive>.*)$")
RECOVERY_FAILED_RE = re.compile(r"daily recovery failed")
CACHE_CLEARED_RE = re.compile(r"daily cache cleared: dir=(?P<dir>.*); removed=(?P<removed>\d+) wav")
CACHE_CLEAR_FAILED_RE = re.compile(r"daily cache clear failed: (?P<error>.*)$")
# 日中の全置換 import（Grid history -> Daily DAW）と、その掃除。
# **`daily cache cleared:` とは別の綴りにしてある。** 同じ綴りにすると、掃除を
# 失った rollover を全置換の掃除が埋め合わせて見えてしまう。
GRID_IMPORT_RE = re.compile(r"Grid history を全置換import: (?P<summary>.*)$")
GRID_IMPORT_CLEARED_RE = re.compile(
    r"grid import cache cleared: dir=(?P<dir>.*); removed=(?P<removed>\d+) wav"
)
GRID_IMPORT_CLEAR_FAILED_RE = re.compile(r"grid import cache clear failed: (?P<error>.*)$")
OFFLINE_RENDER_RE = re.compile(r"offline-render: .*?request_id=(?P<id>\d+).*?mml_hash=(?P<hash>\d+)")
PREPARE_RE = re.compile(
    r"shm-patch-prepare event=start instance=(?P<instance>\d+) .*?"
    r"track(?P<row>\d+)_meas(?P<measure>\d+)\.wav"
)
MEASURE_RE = re.compile(
    r"meas(?P<measure>\d+): live-cache slot=(?P<slot>\d+) .*?sent=(?P<sent>\S*)"
    r"(?: silent=(?P<silent>\S*))?"
)
ROW_RE = re.compile(r"row(\d+)")


class LogSession:
    """1 起動ぶんのログから、突き合わせに要るものだけを抜いたもの。

    `preamble` は **`=== DAW mode ready ===` より前**の startup 行。
    `daily rollover:` は `DawApp::new_with()` の中（＝ready 行より前）で吐かれるので、
    そこだけは前を向いて拾う必要がある。**preamble からは rollover 系しか読まない。**
    演奏のログ（`meas*` / `shm-patch-prepare`）まで前から拾うと、
    **前のセッションの演奏を今のセッションのものとして数えてしまう。**
    """

    def __init__(
        self,
        index: int,
        total: int,
        started_at: str,
        lines: list[str],
        preamble: list[str],
    ):
        self.index = index
        self.total = total
        self.started_at = started_at
        self.rollover: dict | None = None
        self.cache_cleared: dict | None = None
        self.cache_clear_failed: str | None = None
        self.grid_imports: list[str] = []
        self.grid_import_cleared: list[dict] = []
        self.grid_import_clear_failed: list[str] = []
        self.recovery_failed = 0
        self.render_requests: "OrderedDict[str, str]" = OrderedDict()
        self.prepared: list[tuple[int, int]] = []
        self.measures: list[dict] = []
        for line in preamble:
            self._scan_daily(line)
        for line in lines:
            self._scan(line)

    def _scan_daily(self, line: str) -> None:
        match = ROLLOVER_RE.search(line)
        if match:
            self.rollover = {
                "from": match.group("from"),
                "to": match.group("to").strip(),
                "archive": match.group("archive").strip(),
            }
        match = CACHE_CLEARED_RE.search(line)
        if match:
            self.cache_cleared = {
                "dir": match.group("dir").strip(),
                "removed": int(match.group("removed")),
            }
        match = CACHE_CLEAR_FAILED_RE.search(line)
        if match:
            self.cache_clear_failed = match.group("error").strip()
        if RECOVERY_FAILED_RE.search(line):
            self.recovery_failed += 1

    def _scan(self, line: str) -> None:
        self._scan_daily(line)
        self._scan_grid_import(line)
        match = OFFLINE_RENDER_RE.search(line)
        if match:
            self.render_requests[match.group("id")] = match.group("hash")
        match = PREPARE_RE.search(line)
        if match:
            self.prepared.append((int(match.group("row")), int(match.group("measure"))))
        match = MEASURE_RE.search(line)
        if match:
            self.measures.append(
                {
                    "measure": int(match.group("measure")),
                    "slot": int(match.group("slot")),
                    "sent_rows": [int(row) for row in ROW_RE.findall(match.group("sent") or "")],
                    "silent_rows": [
                        int(row) for row in ROW_RE.findall(match.group("silent") or "")
                    ],
                }
            )

    def _scan_grid_import(self, line: str) -> None:
        """全置換 import は ready 行の**あと**に出る（画面へ入ってから押す操作）。

        だから `_scan_daily` と違い preamble からは読まない。
        """
        match = GRID_IMPORT_RE.search(line)
        if match:
            self.grid_imports.append(match.group("summary").strip())
        match = GRID_IMPORT_CLEARED_RE.search(line)
        if match:
            self.grid_import_cleared.append(
                {"dir": match.group("dir").strip(), "removed": int(match.group("removed"))}
            )
        match = GRID_IMPORT_CLEAR_FAILED_RE.search(line)
        if match:
            self.grid_import_clear_failed.append(match.group("error").strip())

    def sent_rows(self) -> set[int]:
        rows: set[int] = set()
        for entry in self.measures:
            rows.update(entry["sent_rows"])
        return rows

    def silent_rows(self) -> set[int]:
        rows: set[int] = set()
        for entry in self.measures:
            rows.update(entry["silent_rows"])
        return rows

    def prepared_rows(self) -> set[int]:
        return {row for row, _ in self.prepared}


def split_sessions(path: Path, session: int) -> LogSession:
    """ログを `=== DAW mode ready ===` で区切って、指定のセッションだけ返す。

    200MB のログを丸ごと保持しないよう、起点の行番号だけ先に集めて 2 度読みする。
    """
    starts: list[tuple[int, str]] = []
    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for number, line in enumerate(handle):
            match = SESSION_RE.match(line)
            if match:
                starts.append((number, match.group("ts")))
    if not starts:
        raise ValueError(f"'=== DAW mode ready ===' が 1 行も無い: {path}")
    index = session if session >= 0 else len(starts) + session
    if not 0 <= index < len(starts):
        raise ValueError(f"セッション {session} は範囲外（全 {len(starts)} セッション）")
    begin = starts[index][0]
    end = starts[index + 1][0] if index + 1 < len(starts) else None
    # 1 つ前の ready 行から今の ready 行までが startup の preamble。
    # `daily rollover:` はここに出る（ready 行より前に吐かれる）。
    preamble_begin = starts[index - 1][0] + 1 if index > 0 else 0
    lines: list[str] = []
    preamble: list[str] = []
    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for number, line in enumerate(handle):
            if number < preamble_begin:
                continue
            if number < begin:
                preamble.append(line)
                continue
            if end is not None and number >= end:
                break
            lines.append(line)
    return LogSession(index, len(starts), starts[index][1], lines, preamble)


def compact_pairs(pairs: list[tuple[int, int]]) -> str:
    names = [f"track{row}_meas{measure}.wav" for row, measure in pairs]
    unique = sorted(set(names))
    if len(unique) <= 2:
        return ", ".join(unique)
    return f"{unique[0]} .. {unique[-1]} ({len(unique)} files)"


def format_rows(rows: set[int]) -> str:
    return ", ".join(f"row{row}" for row in sorted(rows)) if rows else "（無し）"


def check_log(path: Path, session_index: int, project, result: dict) -> bool:
    """ログ 1 セッションぶんを表に出して、project との差を数で出す。

    `project` は `check_daw_cache_staleness.Project`（`None` なら突き合わせは省略）。
    戻り値は「差が無い（OK）か」。
    """
    print()
    print(f"== 実行ログの突き合わせ: {path}")
    session = split_sessions(path, session_index)
    position = session.index - session.total
    print(
        f"   セッション: {session.started_at} 起点"
        f"（最後から {-position} 番目 / 全 {session.total} セッション）"
    )
    if session.rollover:
        print(
            f"   daily rollover: {session.rollover['from']} -> {session.rollover['to']}"
            f"  archive={session.rollover['archive']}"
        )
    else:
        print("   daily rollover: 無し（Resume の日）")
    if session.cache_cleared:
        print(
            f"   daily cache cleared: removed={session.cache_cleared['removed']} wav"
            f"  dir={session.cache_cleared['dir']}"
        )
    elif session.rollover:
        # rollover したのに掃除の行が無い＝前日の WAV が今日のセル名を占めたまま。
        print("   !! rollover したのに daily cache cleared の行が無い   NG")
    if session.cache_clear_failed:
        print(f"   !! daily cache clear failed: {session.cache_clear_failed}   NG")
    if session.recovery_failed:
        # 失敗すると黙って fresh start へ落ちるので、通ったつもりの経路と違うものを見ている。
        print(f"   !! daily recovery failed が {session.recovery_failed} 行   NG")
    for index, summary in enumerate(session.grid_imports):
        print(f"   Grid history を全置換import: {summary}")
        if index < len(session.grid_import_cleared):
            cleared = session.grid_import_cleared[index]
            print(
                f"   grid import cache cleared: removed={cleared['removed']} wav"
                f"  dir={cleared['dir']}"
            )
        else:
            # 全置換したのに掃除の行が無い＝前の曲の WAV が今日のセル名を占めたまま。
            print("   !! 全置換import したのに grid import cache cleared の行が無い   NG")
    for error in session.grid_import_clear_failed:
        print(f"   !! grid import cache clear failed: {error}   NG")
    print(
        f"   offline-render: request {len(session.render_requests)} 件"
        f"（mml_hash {len(set(session.render_requests.values()))} 種）"
    )
    print(
        f"   shm-patch-prepare: {len(session.prepared)} 回 "
        f"{compact_pairs(session.prepared)}"
    )
    print(f"   meas*: live-cache: {len(session.measures)} 小節ぶん")

    result.update(
        {
            "log": str(path),
            "session_started_at": session.started_at,
            "session_index": session.index,
            "session_count": session.total,
            "rollover": session.rollover,
            "cache_cleared": session.cache_cleared,
            "cache_clear_failed": session.cache_clear_failed,
            "grid_imports": session.grid_imports,
            "grid_import_cleared": session.grid_import_cleared,
            "grid_import_clear_failed": session.grid_import_clear_failed,
            "recovery_failed": session.recovery_failed,
            "render_requests": len(session.render_requests),
            "prepared_files": sorted(
                {f"track{row}_meas{measure}.wav" for row, measure in session.prepared}
            ),
            "measure_lines": len(session.measures),
            "sent_rows": sorted(session.sent_rows()),
            "silent_rows": sorted(session.silent_rows()),
            "problems": [],
        }
    )
    ok = session.recovery_failed == 0
    if session.recovery_failed:
        result["problems"].append("daily-recovery-failed")
    if session.rollover and session.cache_cleared is None:
        result["problems"].append("rollover-without-cache-clear")
        ok = False
    if session.cache_clear_failed:
        result["problems"].append("daily-cache-clear-failed")
        ok = False
    if len(session.grid_import_cleared) < len(session.grid_imports):
        result["problems"].append("grid-import-without-cache-clear")
        ok = False
    if session.grid_import_clear_failed:
        result["problems"].append("grid-import-cache-clear-failed")
        ok = False
    if not session.measures:
        print("   !! 演奏の小節ログが 1 行も無い（この起動では鳴らしていない）   NG")
        result["problems"].append("no-measure-lines")
        return False
    if project is None:
        print("   （project が無いので sent= の突き合わせは省略）")
        return ok
    return check_sent_rows(session, project, result) and ok


def check_sent_rows(session: LogSession, project, result: dict) -> bool:
    """**Stage 4 の芯。** `sent=` の行集合と project の中身のある行集合を引き算する。"""
    print("== ログの sent= と project の突き合わせ")
    sent = session.sent_rows()
    content = project.rows_with_content()
    ghost = sorted(sent - content)
    missing = sorted(content - sent)
    print(f"   project: {project.path}  page_date={project.page_date}")
    print(f"   sent= に出た行            : {format_rows(sent)}   ({len(sent)} 行)")
    print(f"   project に中身のある行    : {format_rows(content)}   ({len(content)} 行)")
    print(f"   silent= に落ちた行        : {format_rows(session.silent_rows())}")
    result["project_rows_with_content"] = sorted(content)
    result["ghost_rows"] = ghost
    result["missing_rows"] = missing
    ok = True
    if ghost:
        print(
            f"   project に 1 文字も無いのに鳴っている行: {format_rows(set(ghost))}"
            f"   ({len(ghost)} 行)   NG"
        )
        result["problems"].append("sent-row-without-project-content")
        ok = False
    else:
        print("   project に無いのに鳴っている行は無し   OK")
    if missing:
        # 逆向き。中身があるのに鳴っていない＝キャッシュがまだ焼けていない、など。
        print(
            f"   project に中身があるのに鳴っていない行: {format_rows(set(missing))}"
            f"   ({len(missing)} 行)   NG"
        )
        result["problems"].append("project-content-never-sent")
        ok = False
    ghost_files = sorted(
        {
            f"track{row}_meas{measure}.wav"
            for row, measure in session.prepared
            if not project.cell_has_content(row, measure)
        }
    )
    result["ghost_prepared_files"] = ghost_files
    if ghost_files:
        print(
            f"   project に無いのにロードした WAV: {compact_pairs(session.prepared)} のうち "
            f"{len(ghost_files)} 件   NG"
        )
        result["problems"].append("prepared-wav-without-project-content")
        ok = False
    return ok
