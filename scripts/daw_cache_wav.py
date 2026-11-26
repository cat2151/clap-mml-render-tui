"""キャッシュ WAV の RIFF を読んでフォーマットとフレーム数を返す（`check_daw_cache_staleness.py` 用）。"""

from __future__ import annotations

import struct
from pathlib import Path


class WavError(Exception):
    pass


def read_wav_info(path: Path) -> dict:
    """RIFF を実際に読んでフォーマットとフレーム数を返す。

    **ファイルサイズを 8 で割るだけで済ませないこと。** ヘッダは
    `WAVE_FORMAT_EXTENSIBLE`（fmt チャンク 40 バイト）で 68 バイトあるが、
    そこを前提にすると別の書き手の WAV で静かに間違える。
    """
    with path.open("rb") as handle:
        header = handle.read(12)
        if len(header) < 12 or header[0:4] != b"RIFF" or header[8:12] != b"WAVE":
            raise WavError("RIFF/WAVE ではない")
        fmt: tuple | None = None
        data_bytes: int | None = None
        while True:
            chunk = handle.read(8)
            if len(chunk) < 8:
                break
            chunk_id = chunk[0:4]
            chunk_size = struct.unpack("<I", chunk[4:8])[0]
            if chunk_id == b"fmt ":
                body = handle.read(chunk_size)
                if len(body) < 16:
                    raise WavError("fmt チャンクが短い")
                fmt = struct.unpack("<HHIIHH", body[:16])
            elif chunk_id == b"data":
                # data のサイズはヘッダの申告値。実体が足りない（＝書き込み中に
                # 読んだ）なら、そこが分かるように弾く。
                start = handle.tell()
                actual = max(0, path.stat().st_size - start)
                if chunk_size > actual:
                    raise WavError(f"data が申告より短い（申告 {chunk_size} / 実体 {actual}）")
                if chunk_size == 0:
                    # **本物の「書き込み途中」はこの形で見える**（Stage 5 で実測）。
                    # 書き手（play server の `write_wav` → hound `WavWriter`）は
                    # data チャンク長を **0 で書き出し、`finalize()` のときだけ**
                    # 本当の長さへ書き戻す。だから途中経過は「申告より短い」ではなく
                    # **「申告 0 なのに本体が在る」**になる。上の分岐だけでは拾えない
                    # （申告のほうが小さいので）。
                    raise WavError(
                        f"data チャンク長が 0（書き込み途中。本体は {actual} バイト在る）"
                        if actual > 0
                        else "data チャンク長が 0（音が 1 フレームも入っていない）"
                    )
                data_bytes = chunk_size
                break
            else:
                handle.seek(chunk_size + (chunk_size & 1), 1)
        if fmt is None:
            raise WavError("fmt チャンクが無い")
        if data_bytes is None:
            raise WavError("data チャンクが無い")
    _tag, channels, sample_rate, _byte_rate, block_align, bits = fmt
    if block_align <= 0:
        raise WavError("block_align が 0")
    return {
        "channels": channels,
        "sample_rate": sample_rate,
        "bits": bits,
        "block_align": block_align,
        "frames": data_bytes // block_align,
    }
