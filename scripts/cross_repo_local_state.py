"""`cross_repo_local.py` の定数・小道具と、repo の状態の取得（読むだけで何も変えない）。"""

from __future__ import annotations

import subprocess
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SIBLING_REL = "../clap-mml-play-server"
SIBLING_DIR = (REPO_ROOT / SIBLING_REL).resolve()
PS_URL = "https://github.com/cat2151/clap-mml-play-server"
PS_BRANCH = "main"

CARGO_CONFIG = REPO_ROOT / ".cargo" / "config.toml"
CARGO_LOCK = REPO_ROOT / "Cargo.lock"
HOOKS_DIR = REPO_ROOT / ".githooks"
HOOKS_PATH_VALUE = ".githooks"

# [patch] へ書き出す crate。ここが唯一の定義で、存在確認も生成もこの表から行う。
PATCHED_CRATES: list[tuple[str, str]] = [
    ("cmrt-core", "core-lib"),
    ("cmrt-server-config", "server-config"),
]

# 自動生成した config.toml だけを削除対象にするための目印。
# 1 行目に含まれていなければ人が置いた設定とみなして手を出さない。
MARKER = "cross-repo-local-generated"
LEGACY_MARKERS = ("cross_repo_local_on.bat",)


# --- 小道具 ---------------------------------------------------------------


def info(msg: str) -> None:
    print(msg)


def warn(msg: str) -> None:
    print(f"[WARN] {msg}")


def fail(msg: str) -> None:
    print(f"[ERROR] {msg}", file=sys.stderr)
    raise SystemExit(1)


def run(cmd: list[str], cwd: Path = REPO_ROOT, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(cmd, cwd=cwd, text=True, capture_output=True, encoding="utf-8", errors="replace")
    if check and proc.returncode != 0:
        detail = (proc.stderr or proc.stdout or "").strip()
        fail(f"コマンドが失敗しました: {' '.join(cmd)}\n{detail}")
    return proc


def git_out(args: list[str], cwd: Path) -> str | None:
    proc = run(["git", *args], cwd=cwd, check=False)
    return proc.stdout.strip() if proc.returncode == 0 else None


def short(rev: str | None) -> str:
    return rev[:8] if rev else "不明"


# --- 状態の取得 -----------------------------------------------------------


def local_mode_is_on() -> bool:
    return CARGO_CONFIG.exists()


def parse_lock_ps_entries(text: str) -> dict[str, str | None]:
    """Cargo.lock の中身から play-server 由来 package を {name: rev} で返す。

    rev が None のものは [patch] でローカルパスへ差し替わっている（= ローカルモードの痕跡）。
    """
    data = tomllib.loads(text)
    entries: dict[str, str | None] = {}
    patched_names = {name for name, _ in PATCHED_CRATES}
    for pkg in data.get("package", []):
        name = pkg.get("name", "")
        source = pkg.get("source")
        if source and source.startswith(f"git+{PS_URL}"):
            entries[name] = source.split("#", 1)[1] if "#" in source else ""
        elif source is None and name in patched_names:
            entries[name] = None
    return entries


def lock_ps_entries() -> dict[str, str | None]:
    return parse_lock_ps_entries(CARGO_LOCK.read_text(encoding="utf-8"))


def lock_has_path_entries() -> bool:
    """worktree の Cargo.lock に path 参照（source 行が剥がれた package）が残っているか。"""
    try:
        return any(rev is None for rev in lock_ps_entries().values())
    except (OSError, tomllib.TOMLDecodeError):
        return False


def staged_lock_ps_entries() -> dict[str, str | None]:
    """index（= 次の commit に載る中身）の Cargo.lock を読む。読めなければ止まる。"""
    proc = run(["git", "show", ":Cargo.lock"], check=False)
    if proc.returncode != 0:
        fail("index に Cargo.lock がありません。git status を確認してください。")
    try:
        return parse_lock_ps_entries(proc.stdout)
    except tomllib.TOMLDecodeError as exc:
        fail(f"index の Cargo.lock を解析できませんでした: {exc}")
        raise  # 到達しない（fail が SystemExit を投げる）


def staged_lock_has_path_entries() -> bool:
    """index（= 次の commit に載る中身）の Cargo.lock に path 参照が混ざっているか。

    worktree だけ直しても commit されるのは index なので、ここを別に見ないと素通りする。
    """
    proc = run(["git", "show", ":Cargo.lock"], check=False)
    if proc.returncode != 0:
        return False
    try:
        return any(rev is None for rev in parse_lock_ps_entries(proc.stdout).values())
    except tomllib.TOMLDecodeError:
        return False


def global_hooks_path_set() -> bool:
    """global の core.hooksPath が設定されているか。値は見ない（この repo の外の事情なので）。"""
    proc = run(["git", "config", "--global", "--get", "core.hooksPath"], check=False)
    return bool(proc.stdout.strip())


def hooks_enabled() -> bool:
    """local の core.hooksPath が .githooks か、global に core.hooksPath があれば有効。

    global 側の hook は自分の処理のあと repo の .githooks/pre-commit を呼ぶ約束なので、
    それがあれば local を向けなくても cross-repo 判定は走る。
    """
    proc = run(["git", "config", "--get", "core.hooksPath"], check=False)
    if proc.stdout.strip() == HOOKS_PATH_VALUE:
        return True
    return global_hooks_path_set()


def sibling_revs() -> tuple[str | None, str | None, bool]:
    """兄弟 repo の (ローカル HEAD, origin/main, HEAD が push 済みか) を返す。"""
    if not SIBLING_DIR.exists():
        return None, None, False
    head = git_out(["rev-parse", "HEAD"], SIBLING_DIR)
    remote = git_out(["rev-parse", f"origin/{PS_BRANCH}"], SIBLING_DIR)
    pushed = False
    if head and remote:
        proc = run(["git", "merge-base", "--is-ancestor", head, remote], cwd=SIBLING_DIR, check=False)
        pushed = proc.returncode == 0
    return head, remote, pushed


def sibling_worktree_is_clean() -> bool:
    """兄弟 repo に commit されていない変更が無いか。取得失敗も安全側で dirty とみなす。"""
    proc = run(["git", "status", "--porcelain"], cwd=SIBLING_DIR, check=False)
    return proc.returncode == 0 and not proc.stdout.strip()


def fetch_sibling() -> None:
    proc = run(["git", "fetch", "--quiet", "origin", PS_BRANCH], cwd=SIBLING_DIR, check=False)
    if proc.returncode != 0:
        warn("兄弟 repo の git fetch に失敗しました。origin/main はローカルのキャッシュを使います。")


def generated_by_us() -> bool:
    try:
        first = CARGO_CONFIG.read_text(encoding="utf-8").splitlines()[0]
    except (OSError, IndexError):
        return False
    return MARKER in first or any(m in first for m in LEGACY_MARKERS)
