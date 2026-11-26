"""`daily_daw/current.json` を、グリッドの行 index で引ける形に読む（`check_daw_cache_staleness.py` 用）。"""

from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path

# グリッド行 index の割り当て（daw/src/tracks.rs）。
TEMPO_TRACK = 0
CHORD_TRACK = 1
FIRST_PLAYABLE_TRACK = 2
FIRST_SAVED_PLAYABLE_TRACK = 1


def grid_row_from_saved_track(saved_track: int) -> int:
    """保存ファイルの track 番号 → グリッドの行 index（`daw/src/tracks.rs` と同じ規則）。"""
    if saved_track == TEMPO_TRACK:
        return TEMPO_TRACK
    return saved_track + (FIRST_PLAYABLE_TRACK - FIRST_SAVED_PLAYABLE_TRACK)


def strip_json_prefix(cell: str) -> str:
    """セル先頭の JSON を落として MML 本体だけにする（`split_mml_fragment` 相当）。"""
    text = cell.strip()
    if not text.startswith("{"):
        return text
    depth = 0
    in_str = False
    escaped = False
    for index, char in enumerate(text):
        if in_str:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_str = False
            continue
        if char == '"':
            in_str = True
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[index + 1 :]
    return ""


def json_prefix(cell: str) -> dict:
    """セル先頭の JSON を dict で返す。JSON が無い／壊れていれば空 dict。"""
    text = cell.strip()
    if not text.startswith("{"):
        return {}
    body = text[: len(text) - len(strip_json_prefix(cell))]
    try:
        value = json.loads(body)
    except json.JSONDecodeError:
        return {}
    return value if isinstance(value, dict) else {}


class Project:
    """`current.json` の中身を、グリッドの行 index で引ける形に均したもの。"""

    def __init__(self, path: Path, raw: dict):
        self.path = path
        self.page_date: str | None = raw.get("page_date")
        project = ((raw.get("project_file") or {}).get("project")) or {}
        self.cells: dict[int, dict[int, str]] = defaultdict(dict)
        for track in project.get("tracks") or []:
            row = grid_row_from_saved_track(int(track.get("track_index", 0)))
            for cell in track.get("non_empty_cells") or []:
                self.cells[row][int(cell.get("measure_index", 0))] = cell.get("mml") or ""
        for cell in ((project.get("chord_track") or {}).get("non_empty_cells")) or []:
            self.cells[CHORD_TRACK][int(cell.get("measure_index", 0))] = cell.get("mml") or ""
        self.cached_measures = [
            (int(entry.get("track", -1)), int(entry.get("measure", -1)))
            for entry in raw.get("cached_measures") or []
        ]

    def conductor_text(self) -> str:
        cells = self.cells.get(TEMPO_TRACK, {})
        return "".join(strip_json_prefix(cells[measure]) for measure in sorted(cells))

    def beat_numerator(self) -> int:
        """`{"beat": "4/4"}` → 4。読めなければ 4（`parse_beat_numerator` と同じ）。"""
        for measure in sorted(self.cells.get(TEMPO_TRACK, {})):
            beat = json_prefix(self.cells[TEMPO_TRACK][measure]).get("beat")
            if isinstance(beat, str):
                head = beat.split("/")[0]
                if head.isdigit():
                    return max(1, int(head))
        return 4

    def bpm(self) -> float:
        """conductor 行を繋いだ本体から最初の `t<整数>` を拾う（`parse_tempo_bpm` と同じ）。"""
        match = re.search(r"t(\d+)", self.conductor_text())
        bpm = float(match.group(1)) if match else 120.0
        return min(960.0, max(1.0, bpm))

    def measure_frames(self, sample_rate: int) -> int:
        """今日の 1 小節のフレーム数（`compute_measure_samples` / 2 と同じ値）。"""
        seconds = self.beat_numerator() * 60.0 / self.bpm()
        interleaved = round(seconds * sample_rate * 2.0)
        interleaved += interleaved & 1
        return interleaved // 2

    def cell_has_content(self, row: int, measure: int) -> bool:
        """そのセルが「鳴る中身」を持つか（`daw/src/mml.rs` の `cell_has_content` 相当）。

        **`non_empty_cells` が空でも鳴ることがある。** init セルに
        `generate from chord track` があれば、その行の各小節は chord 行から生成される。
        """
        if measure == 0 or row < FIRST_PLAYABLE_TRACK:
            return False
        if self.cells.get(row, {}).get(measure, "").strip():
            return True
        init = self.cells.get(row, {}).get(0, "")
        if not json_prefix(init).get("generate from chord track"):
            return False
        return bool(self.cells.get(CHORD_TRACK, {}).get(measure, "").strip())

    def rows_with_content(self) -> set[int]:
        rows = set()
        highest_row = max(self.cells, default=0)
        for row in range(FIRST_PLAYABLE_TRACK, highest_row + 1):
            measures = set(self.cells.get(row, {})) | set(self.cells.get(CHORD_TRACK, {}))
            if any(self.cell_has_content(row, m) for m in measures if m > 0):
                rows.add(row)
        return rows


def load_project(path: Path) -> Project:
    return Project(path, json.loads(path.read_text(encoding="utf-8")))
