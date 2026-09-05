#!/usr/bin/env python3
"""兄弟 repo (clap-mml-play-server) を参照する cross-repo ローカルモードの切り替え。

サブコマンド:
  on      .cargo/config.toml を生成し、git 依存を兄弟 repo の作業ツリーへ向ける
  off     ローカルモードを解除し、Cargo.lock を「push 済みの最新 HEAD」へ張り直す
  status  いまの状態を表示する。commit して安全でなければ非 0 で終了する
  hooks   pre-commit hook を有効化する（core.hooksPath を .githooks へ向ける）

背景と設計判断は docs/adr/0010-two-repo-layout.md を参照。
"""

from __future__ import annotations

import argparse
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


def hooks_enabled() -> bool:
    proc = run(["git", "config", "--get", "core.hooksPath"], check=False)
    return proc.stdout.strip() == HOOKS_PATH_VALUE


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


def fetch_sibling() -> None:
    proc = run(["git", "fetch", "--quiet", "origin", PS_BRANCH], cwd=SIBLING_DIR, check=False)
    if proc.returncode != 0:
        warn("兄弟 repo の git fetch に失敗しました。origin/main はローカルのキャッシュを使います。")


# --- on -------------------------------------------------------------------


def cmd_on(_args: argparse.Namespace) -> int:
    if not SIBLING_DIR.exists():
        fail(
            f"兄弟 repo が見つかりません: {SIBLING_DIR}\n"
            "        この repo と clap-mml-play-server を同じ親ディレクトリへ並べて置いてください。"
        )
    missing = [d for _, d in PATCHED_CRATES if not (SIBLING_DIR / d / "Cargo.toml").exists()]
    if missing:
        fail(
            f"兄弟 repo に次の crate がありません: {', '.join(missing)}\n"
            "        clap-mml-play-server を最新へ更新してください。"
        )

    if CARGO_CONFIG.exists() and not generated_by_us():
        fail(
            f"{CARGO_CONFIG} は自動生成物ではありません。上書きしないので、内容を確認して手で外してください。"
        )

    lines = [
        f"# {MARKER}: scripts/cross_repo_local.py が生成。commit しないこと（.gitignore 済み）。",
        "# 相対パスはこの config ファイルの位置基準で解決される。",
        f'[patch."{PS_URL}"]',
    ]
    for name, dir_name in PATCHED_CRATES:
        lines.append(f'{name} = {{ path = "{SIBLING_REL}/{dir_name}" }}')

    CARGO_CONFIG.parent.mkdir(parents=True, exist_ok=True)
    CARGO_CONFIG.write_text("\n".join(lines) + "\n", encoding="utf-8")

    info("ローカル横断ビルドを ON にしました。")
    for name, dir_name in PATCHED_CRATES:
        info(f"  {name} を {SIBLING_REL}/{dir_name} から解決します。")
    info("")
    info("この間の Cargo.lock は source 行が剥がれた別物になります。commit しないこと。")
    info("戻すときは cross_repo_local_off.bat を実行してください。")
    if not hooks_enabled():
        warn(
            "pre-commit hook が未設定です。cross_repo_local_hooks.bat を一度実行しておくと、"
            "壊れた Cargo.lock の commit を git 側で止められます。"
        )
    return 0


def generated_by_us() -> bool:
    try:
        first = CARGO_CONFIG.read_text(encoding="utf-8").splitlines()[0]
    except (OSError, IndexError):
        return False
    return MARKER in first or any(m in first for m in LEGACY_MARKERS)


# --- off ------------------------------------------------------------------


def cmd_off(args: argparse.Namespace) -> int:
    if CARGO_CONFIG.exists():
        if not generated_by_us():
            fail(f"{CARGO_CONFIG} は自動生成物ではありません。削除しないので手で確認してください。")
        CARGO_CONFIG.unlink()
        # .cargo が空になったときだけ消える（他の設定を置いていれば残る）。
        try:
            CARGO_CONFIG.parent.rmdir()
        except OSError:
            pass
        info(".cargo/config.toml を削除しました。")
    else:
        info("ローカル横断ビルドは OFF のままです。")

    if args.keep_lock:
        info("--keep-lock 指定のため Cargo.lock の巻き戻しは行いません。")
    else:
        # HEAD 比較でないと、既に git add 済みの lock 差分を「差分なし」と報告してしまう。
        stat = git_out(["diff", "HEAD", "--stat", "--", "Cargo.lock"], REPO_ROOT)
        if stat:
            info("未 commit の Cargo.lock 変更を捨てて HEAD の内容へ戻します:")
            info(f"  {stat.strip()}")
        # `git checkout -- <path>` が復元するのは HEAD ではなく index。ローカルモード中に
        # git add していると、壊れた lock をそのまま書き戻したうえ index も壊れたまま残り、
        # 直後の cargo update が意味の分からないエラーで落ちる。source と index を明示する。
        proc = run(
            ["git", "restore", "--source=HEAD", "--staged", "--worktree", "--", "Cargo.lock"],
            check=False,
        )
        if proc.returncode != 0:
            fail("Cargo.lock を復元できませんでした。git status を確認してください。")
        info("Cargo.lock を HEAD の内容へ戻しました（worktree と index の両方）。")

    # ここが従来の bat に無かった肝。HEAD の lock は「ローカルモードに入る前」の
    # 古い rev を指しているので、そのまま commit すると兄弟 repo の新しい API を
    # 使ったコードがビルドできない。AGENTS.md の「古い lock を放置せず最新 HEAD へ
    # 追従」に従い、必ず張り直す。
    if SIBLING_DIR.exists():
        fetch_sibling()
    info("Cargo.lock を play-server の最新 HEAD へ張り直します（cargo update）…")
    pkgs: list[str] = []
    for name, _ in PATCHED_CRATES:
        pkgs += ["-p", name]
    proc = run(["cargo", "update", *pkgs], check=False)
    output = (proc.stdout + proc.stderr).strip()
    if proc.returncode != 0:
        hint = ""
        if lock_has_path_entries():
            # cargo は "did not match any packages" としか言わないので、こちらで原因を名指しする。
            hint = (
                "\n\n        Cargo.lock がまだ path 参照のままです（source 行が剥がれている）。"
                "\n        復元が効いていないので、次を実行してから off をやり直してください:"
                "\n          git restore --source=HEAD --staged --worktree -- Cargo.lock"
            )
        fail(f"cargo update が失敗しました。\n{output}{hint}")
    for line in output.splitlines():
        if "clap-mml-play-server" in line:
            info(f"  {line.strip()}")

    info("")
    return report_status(after_off=True)


# --- hooks ----------------------------------------------------------------


def cmd_hooks(_args: argparse.Namespace) -> int:
    hook = HOOKS_DIR / "pre-commit"
    if not hook.exists():
        fail(f"{hook} がありません。この repo を最新へ更新してください。")
    if hooks_enabled():
        info(f"core.hooksPath は既に {HOOKS_PATH_VALUE} です。")
    else:
        run(["git", "config", "core.hooksPath", HOOKS_PATH_VALUE])
        info(f"core.hooksPath を {HOOKS_PATH_VALUE} に設定しました。")
    info("以後 git commit のたびに status --no-fetch --staged が走ります（cargo は呼ばない）。")
    info("どうしても通したいときだけ git commit --no-verify。")
    return 0


# --- status ---------------------------------------------------------------


def cmd_status(args: argparse.Namespace) -> int:
    if SIBLING_DIR.exists() and not args.no_fetch:
        fetch_sibling()
    return report_status(after_off=False, staged=args.staged)


def report_status(after_off: bool, staged: bool = False) -> int:
    """staged=True なら worktree ではなく index（commit に載る中身）を判定する。"""
    problems: list[str] = []

    on = local_mode_is_on()
    info(f"ローカル横断モード : {'ON' if on else 'OFF'}")
    if on and not staged:
        problems.append("ローカルモードが ON。この状態の Cargo.lock は commit してはいけない。")
    elif on:
        # 実装中はほぼ常に ON なので、ON それ自体では止めない（止めると hook が煙たがられる）。
        # 危険なのは「壊れた lock / 古い lock が commit に載ること」で、それは下の 2 つが捕まえる。
        warn("ローカルモードが ON のままです。commit に載る Cargo.lock（index）だけを判定します。")

    entries = staged_lock_ps_entries() if staged else lock_ps_entries()
    if not entries:
        problems.append("Cargo.lock に play-server 由来の package が見つからない。")
    revs = {rev for rev in entries.values() if rev}
    patched = sorted(name for name, rev in entries.items() if rev is None)
    if patched:
        problems.append(
            f"Cargo.lock の {', '.join(patched)} が path 参照のまま（source 行が剥がれている）。"
            " 直すには git restore --source=HEAD --staged --worktree -- Cargo.lock のあと off。"
        )
    if not staged and staged_lock_has_path_entries():
        problems.append(
            "git add 済みの Cargo.lock が path 参照のまま。commit に載るのは index の方なので、"
            "worktree だけ直しても素通りする。"
        )
    if len(revs) > 1:
        problems.append(f"Cargo.lock の rev がばらついている: {', '.join(sorted(short(r) for r in revs))}")
    lock_rev = next(iter(revs)) if len(revs) == 1 else None
    where = "index" if staged else "worktree"
    info(f"Cargo.lock の rev  : {short(lock_rev)} ({len(entries)} package / {where})")

    head, remote, pushed = sibling_revs()
    if head is None:
        info("兄弟 repo          : 見つからない（git 依存のみで運用中）")
    else:
        info(f"play-server HEAD   : {short(head)}")
        info(f"play-server origin : {short(remote)}{'' if pushed else '  ← HEAD が未 push'}")
        if not pushed:
            problems.append(
                "play-server のローカル HEAD が push されていない。"
                "先に play-server を push しないと、ローカルモードを使わない環境でビルドが壊れる。"
            )
        elif lock_rev and remote and lock_rev != remote:
            remedy = "`cargo update -p cmrt-core -p cmrt-server-config` で追従すること。"
            if staged:
                remedy += " 追従済みなら git add Cargo.lock を忘れている。"
            problems.append(
                f"Cargo.lock の rev ({short(lock_rev)}) が play-server の origin/{PS_BRANCH} "
                f"({short(remote)}) より古い。{remedy}"
            )

    info("")
    if problems:
        for p in problems:
            print(f"[NG] {p}")
        info("")
        info("commit して安全な状態ではありません。")
        return 1

    info("[OK] commit して安全な状態です。")
    if after_off:
        info("次: cargo build --workspace && cargo test --workspace")
    return 0


# --- entry point ----------------------------------------------------------


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")

    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("on", help="ローカル横断ビルドを ON にする").set_defaults(func=cmd_on)

    p_off = sub.add_parser("off", help="OFF にして Cargo.lock を最新 HEAD へ張り直す")
    p_off.add_argument(
        "--keep-lock",
        action="store_true",
        help="Cargo.lock を HEAD へ巻き戻さない（ローカルモード中に正当な lock 変更をした場合）",
    )
    p_off.set_defaults(func=cmd_off)

    p_status = sub.add_parser("status", help="状態を表示。commit して安全でなければ非 0 で終了")
    p_status.add_argument("--no-fetch", action="store_true", help="兄弟 repo の git fetch を省く")
    p_status.add_argument(
        "--staged",
        action="store_true",
        help="worktree ではなく index（commit に載る中身）の Cargo.lock を判定する",
    )
    p_status.set_defaults(func=cmd_status)

    sub.add_parser("hooks", help="pre-commit hook を有効化する").set_defaults(func=cmd_hooks)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
