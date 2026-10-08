#!/usr/bin/env python3
"""OSのTemp直下にある cmrt* ファイルとディレクトリを一括削除する。

実行: python scripts/cleanup_test_temp_dirs.py
ジャンクションとシンボリックリンクは削除しない。Python 3.12以上。
"""

import shutil
import tempfile
from pathlib import Path


def main():
    for path in Path(tempfile.gettempdir()).glob("cmrt*"):
        if path.is_symlink() or path.is_junction():
            continue
        print(f"削除: {path}", flush=True)
        if path.is_dir():
            shutil.rmtree(path)
        else:
            path.unlink()


if __name__ == "__main__":
    main()
