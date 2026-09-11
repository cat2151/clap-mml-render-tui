#!/usr/bin/env python3
"""テストが「実 %LOCALAPPDATA%」や「プロセス共有 temp」を触っていないかを見る canary。

`rg` でテスト本文の文字列を探す手は**使えない**（呼び出しが 2 段先に隠れていると
1 行も出ない。実際 Stage 6 の犯人は `invalidate_dependent_cells` 経由だった）。
**sentinel を置いて走らせ、消えたかどうかで**判定する。

- canary A: 実 `%LOCALAPPDATA%/clap-mml-render-tui/daw_cache/unknown-plugin/` へ
  sentinel WAV を 8 個置く。8 個とも残れば OK。`unknown-plugin` はテスト専用の
  名前空間（実運用は `Surge XT`）なので実データは壊さない。
- canary B: `%TEMP%/cmrt_test_process_*` を消してから走らせ、その下に
  `daw_cache/` が生えないこと。

**A と B は見えるものが違うので両方要る。** `CMRT_BASE_DIR` が立つ前に走った
guard 無しテストは A に、立ったあとに走ったものは B に出る。

使い方:

    python scripts/canary_test_cache_dirs.py target/debug/deps/cmrt_daw-XXXX.exe ...

実行ファイルのパスは `cargo test -p <crate> --lib --no-run` が印字したものを使うこと
（`ls -t` で選ぶと feature 違いの別物を掴む）。
終了コード 0 = 違反なし。
"""

from __future__ import annotations

import os
import pathlib
import shutil
import subprocess
import sys

SENTINEL_TRACKS = (2, 3, 4, 5)
SENTINEL_MEASURES = (1, 2)


def real_cache_dir() -> pathlib.Path:
    return (
        pathlib.Path(os.environ["LOCALAPPDATA"])
        / "clap-mml-render-tui"
        / "daw_cache"
        / "unknown-plugin"
    )


def place_sentinels(root: pathlib.Path) -> list[pathlib.Path]:
    root.mkdir(parents=True, exist_ok=True)
    placed = []
    for track in SENTINEL_TRACKS:
        for measure in SENTINEL_MEASURES:
            path = root / f"track{track}_meas{measure}.wav"
            path.write_text("S")
            placed.append(path)
    return placed


def clear_process_dirs(temp: pathlib.Path) -> None:
    for path in temp.glob("cmrt_test_process_*"):
        shutil.rmtree(path, ignore_errors=True)


def main() -> int:
    binaries = sys.argv[1:]
    if not binaries:
        print(__doc__)
        return 2
    temp = pathlib.Path(os.environ["TEMP"])
    root = real_cache_dir()
    failed = False
    for binary in binaries:
        sentinels = place_sentinels(root)
        clear_process_dirs(temp)
        subprocess.run(
            [str(pathlib.Path(binary).resolve()), "--test-threads=1"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        survived = sum(1 for path in sentinels if path.exists())
        grown = [str(p) for p in temp.glob("cmrt_test_process_*/*/*/daw_cache")]
        name = pathlib.Path(binary).name
        print(f"{name}: canary A = {survived}/{len(sentinels)} / canary B = {len(grown)}")
        for path in grown:
            print(f"  B: {path}")
        if survived != len(sentinels) or grown:
            failed = True
        for path in sentinels:
            path.unlink(missing_ok=True)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
