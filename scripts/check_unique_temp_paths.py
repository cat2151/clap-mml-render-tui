#!/usr/bin/env python3
"""`std::env::temp_dir()` から作るテスト用パスが、プロセスごとにユニークかを機械で見る。

同じ workspace で `cargo test` を 2 つ走らせる（人間 + agent、agent 2 匹）と、
固定名の temp path は **1 つのディレクトリを取り合って** flaky になる。
1 プロセスの中では起きないので、単独で何回回しても再現しない。

`rg 'temp_dir().join("'` だけでは足りない（`format!` で組み立てた固定名を取り逃がす）。
ここでは `temp_dir()` の**式そのもの**を読んで、次のどれかを含むかで判定する:

- `process::id()`（pid が名前に入っている）
- `unique_test_dir(` / `TempDirGuard::new(` / `temp_local_dirs(`（共通ヘルパ経由）

使い方: python scripts/check_unique_temp_paths.py [--list]
終了コード 0 = 違反なし。
"""

from __future__ import annotations

import pathlib
import re
import sys

SANCTIONED = (
    "process::id()",
    "unique_test_dir(",
    "TempDirGuard::new(",
    "temp_local_dirs(",
)
# 共通ヘルパ自身は pid を組み立てる当人なので対象外。
ALLOWLIST = {pathlib.PurePath("history/src/test_support/temp_dir.rs")}


# 名前を組み立ててから join する書き方（`let unique = format!(..pid..); ..join(unique)`）を
# 拾うため、直前の数行も一緒に見る。
CONTEXT_LINES = 12


def statement_at(text: str, index: int) -> str:
    """`temp_dir()` の文の終わり（`;`）までと、その直前 12 行を返す。"""
    end = text.find(";", index)
    tail = text[index : end if end != -1 else len(text)]
    head = chr(10).join(text[:index].splitlines()[-CONTEXT_LINES:])
    return head + tail


def rust_sources(root: pathlib.Path):
    """`target/` を**降りずに**（walk が数十万ファイルで詰まる）`*.rs` を集める。"""
    import os

    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in ("target", ".git")]
        for name in filenames:
            if name.endswith(".rs"):
                yield pathlib.Path(dirpath) / name


def violations(root: pathlib.Path) -> list[tuple[pathlib.Path, int, str]]:
    found: list[tuple[pathlib.Path, int, str]] = []
    for path in sorted(rust_sources(root)):
        rel = path.relative_to(root)
        if pathlib.PurePath(rel.as_posix()) in ALLOWLIST:
            continue
        text = path.read_text(encoding="utf-8")
        for match in re.finditer(r"temp_dir\(\)", text):
            statement = statement_at(text, match.start())
            joined = statement[statement.index("temp_dir()") :]
            # `starts_with(std::env::temp_dir())` のような「読むだけ」は対象外。
            if ".join(" not in joined:
                continue
            if any(token in statement for token in SANCTIONED):
                continue
            line = text.count("\n", 0, match.start()) + 1
            found.append((rel, line, " ".join(joined.split())[:100]))
    return found


def main() -> int:
    root = pathlib.Path(__file__).resolve().parent.parent
    found = violations(root)
    for rel, line, snippet in found:
        print(f"{rel.as_posix()}:{line}: {snippet}")
    print(f"固定 temp path: {len(found)} 件")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
