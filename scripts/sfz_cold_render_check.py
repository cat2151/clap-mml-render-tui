#!/usr/bin/env python3
"""sforzando の offline render が「OS cache に載っていないサンプル」で途切れるかを測る。

    python scripts/sfz_cold_render_check.py
    python scripts/sfz_cold_render_check.py --trials 3 --parallel 4

## 冷えたサンプルの作り方

同じ音高を一度最後まで鳴らすとサンプルが OS cache に載り、以後は再現しなくなる。
そこで試行ごとに、sfz と参照サンプルを `robocopy /J`（unbuffered I/O）で
ARIA の `user_files_dir` の下へ複製する。unbuffered に書いたファイルは OS cache に
載らないので、毎回同じ MML で「冷えたサンプル」を用意できる。複製は試行後に消す。
複製先は `user_files_dir` の下でなければならない（render server が user bank の外の
sfz を拒否する）。

## 判定

既定の MML（t124 の全音符 4 声）に合わせた閾値:

- `last`: 左 ch がピーク -40dB 以上だった最後の位置（秒）。1.5 秒未満なら途切れ
- `ratio`: 1.0-1.8 秒の RMS / 0.2-0.5 秒の RMS。一部の声部だけ切れても下がる。
  切れていない render は 0.83-1.09 だった。0.5 未満なら途切れ

1 本でも途切れたら終了コード 1。
"""

from __future__ import annotations

import argparse
import io
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
import wave
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_SERVER = (
    REPO_ROOT.parent
    / "clap-mml-play-server"
    / "target"
    / "release"
    / "clap-mml-render-server.exe"
)
DEFAULT_SFZ = "Virtual-Playing-Orchestra3/Strings/all-strings-SEC-normal-mod-wheel.sfz"
DEFAULT_MML = "t124o5d+1;t124o5f+1;t124o5a+1;t124o6c+1"
COLD_DIR_NAME = "cmrt-cold-sample-check"
CUT_LAST_SEC = 1.5
CUT_RATIO = 0.5


def aria_user_files_dir() -> Path:
    import winreg

    with winreg.OpenKey(
        winreg.HKEY_CURRENT_USER, r"Software\Plogue Art et Technologie, Inc\Aria"
    ) as key:
        return Path(winreg.QueryValueEx(key, "user_files_dir")[0])


def make_cold_copy(user_root: Path, sfz_rel: str, tag: str) -> tuple[Path, str]:
    """sfz と参照サンプルを unbuffered で複製し、(複製先, 複製した sfz の絶対 path) を返す。"""
    sfz = user_root / sfz_rel
    top = (
        sfz.parent.parent
    )  # sample= は `..\libs\...` のように sfz の 1 つ上を基点にする
    dst = user_root / COLD_DIR_NAME / tag
    text = sfz.read_text(encoding="latin1")
    by_dir: dict[Path, list[str]] = {}
    for sample in {m.strip() for m in re.findall(r"^sample=(.*)$", text, re.M)}:
        path = (sfz.parent / sample.replace("\\", "/")).resolve()
        by_dir.setdefault(path.parent.relative_to(top.resolve()), []).append(path.name)
    by_dir.setdefault(sfz.parent.relative_to(top), []).append(sfz.name)
    for rel_dir, names in by_dir.items():
        for i in range(0, len(names), 40):
            result = subprocess.run(
                ["robocopy", str(top / rel_dir), str(dst / rel_dir), *names[i : i + 40]]
                + ["/J", "/NJH", "/NJS", "/NP", "/NFL", "/NDL"],
                capture_output=True,
            )
            if result.returncode >= 8:
                raise RuntimeError(f"robocopy failed: {result.stdout!r}")
    # 絶対 path を渡すと、render server は音色置き場の基点を無視してそのまま使う。
    return dst, str(dst / sfz.relative_to(top))


class RenderServer:
    """render server を専用 port で起動する。stdin を閉じると server も終わる。"""

    def __init__(self, exe: Path, port: int, workers: int, scratch: Path):
        local = Path(os.environ["LOCALAPPDATA"]) / "clap-mml-render-tui" / "config.toml"
        config = scratch / f"config_{port}.toml"
        config.write_text(
            f"offline_render_server_port = {port}\n"
            f"offline_render_server_workers = {workers}\n"
            + local.read_text(encoding="utf-8"),
            encoding="utf-8",
        )
        env = dict(os.environ, CMRT_RENDER_SERVER_EXIT_ON_STDIN_CLOSE="1")
        self.port = port
        self.lines: list[str] = []
        self.proc = subprocess.Popen(
            [str(exe), "--config", str(config)],
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            env=env,
        )
        ready = threading.Event()

        def pump() -> None:
            for raw in self.proc.stderr:
                line = raw.decode("utf-8", "replace").rstrip()
                self.lines.append(line)
                if "listening on" in line:
                    ready.set()

        threading.Thread(target=pump, daemon=True).start()
        if not ready.wait(60):
            self.close()
            raise RuntimeError(
                "render server did not start:\n" + "\n".join(self.lines[-20:])
            )

    def render(self, patch: str, mml: str) -> bytes:
        body = json.dumps({"Surge XT patch": patch, "beat": "4/4"}) + mml
        request = urllib.request.Request(
            f"http://127.0.0.1:{self.port}/render",
            data=body.encode("utf-8"),
            headers={"Content-Type": "text/plain; charset=utf-8"},
        )
        last_error: Exception | None = None
        # 送信中に接続が切られることがあった（server 側は request を受け取っていない）。
        for _ in range(3):
            try:
                with urllib.request.urlopen(request, timeout=600) as response:
                    return response.read()
            except urllib.error.URLError as error:
                last_error = error
                time.sleep(0.5)
        raise RuntimeError(f"render failed: {last_error}")

    def render_ms(self, count: int) -> list[int]:
        found = [re.search(r"render_ms=(\d+)", line) for line in self.lines]
        return [int(m.group(1)) for m in found if m][-count:]

    def close(self) -> None:
        try:
            self.proc.stdin.close()
            self.proc.wait(15)
        except Exception:
            self.proc.kill()
            self.proc.wait()


def left_channel(wav_bytes: bytes) -> tuple[list[int], int]:
    with wave.open(io.BytesIO(wav_bytes)) as w:
        frames, channels, rate = w.getnframes(), w.getnchannels(), w.getframerate()
        samples = struct.unpack(f"<{frames * channels}h", w.readframes(frames))
    return list(samples[0::channels]), rate


def measure(wav_bytes: bytes) -> dict:
    left, rate = left_channel(wav_bytes)
    peak = max((abs(x) for x in left), default=0)
    threshold = peak * 0.01
    last = next(
        (i for i in range(len(left) - 1, -1, -1) if abs(left[i]) >= threshold), 0
    )

    def rms(t0: float, t1: float) -> float:
        seg = left[int(t0 * rate) : int(t1 * rate)]
        return (sum(x * x for x in seg) / max(1, len(seg))) ** 0.5

    head = rms(0.2, 0.5)
    ratio = rms(1.0, 1.8) / head if head else 0.0
    last_sec = last / rate if peak else 0.0
    return {
        "last": round(last_sec, 3),
        "ratio": round(ratio, 3),
        "cut": last_sec < CUT_LAST_SEC or ratio < CUT_RATIO,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--sfz", default=DEFAULT_SFZ, help="user_files_dir からの相対 path"
    )
    parser.add_argument("--mml", default=DEFAULT_MML)
    parser.add_argument("--trials", type=int, default=3)
    parser.add_argument(
        "--parallel", type=int, default=1, help="同時に投げる request 数"
    )
    parser.add_argument("--server", type=Path, default=DEFAULT_SERVER)
    parser.add_argument("--port", type=int, default=47931)
    parser.add_argument("--settle", type=float, default=3.0, help="複製後に待つ秒数")
    args = parser.parse_args()

    user_root = aria_user_files_dir()
    scratch = Path(os.environ.get("TEMP", ".")) / "cmrt-sfz-cold-render-check"
    scratch.mkdir(parents=True, exist_ok=True)
    total = cut = 0
    for trial in range(args.trials):
        copies = [
            make_cold_copy(user_root, args.sfz, f"t{trial}_{j}_{os.getpid()}")
            for j in range(args.parallel)
        ]
        try:
            time.sleep(args.settle)
            server = RenderServer(
                args.server, args.port, max(4, args.parallel), scratch
            )
            try:
                with ThreadPoolExecutor(args.parallel) as pool:
                    wavs = list(
                        pool.map(lambda c: server.render(c[1], args.mml), copies)
                    )
                render_ms = server.render_ms(args.parallel)
            finally:
                server.close()
        finally:
            for dst, _ in copies:
                shutil.rmtree(dst, ignore_errors=True)
        rows = [measure(w) for w in wavs]
        total += len(rows)
        cut += sum(r["cut"] for r in rows)
        print(
            json.dumps({"trial": trial, "results": rows, "render_ms": render_ms}),
            flush=True,
        )
    cold_root = user_root / COLD_DIR_NAME
    if cold_root.is_dir() and not any(cold_root.iterdir()):
        cold_root.rmdir()
    print(f"cut {cut}/{total}")
    return 1 if cut else 0


if __name__ == "__main__":
    sys.exit(main())
