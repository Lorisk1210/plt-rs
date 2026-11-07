"""Print the zlib-ng streams pltrs-codec's tests pin (tests/zlib_ng.rs).

pltrs's DEFLATE encoder writes zlib-ng 2.3's bytes. This script asks a
zlib-ng 2.3.3 build for them, through `zng_ref.c` (next to it):

    git clone --depth 1 --branch 2.3.3 https://github.com/zlib-ng/zlib-ng
    cd zlib-ng && ./configure --zlib-compat --static && make libz.a
    cc -O2 -I. path/to/zng_ref.c libz.a -o zng_ref

then, with Pillow installed (it filters the images' rows, whose
compression the PNG tables pin):

    python expected.py path/to/zng_ref

and paste the output into tests/zlib_ng.rs.
"""

import io
import struct
import subprocess
import sys
import zlib
from pathlib import Path

from PIL import Image

CORPUS = Path(__file__).resolve().parents[2] / "tests/fixtures/zlib/corpus.bin"

# zlib's strategy numbers, by the names of pltrs's `Strategy`.
STRATEGIES = {"Default": 0, "Filtered": 1, "HuffmanOnly": 2, "Rle": 3, "Fixed": 4}


def run(harness, args, data):
    result = subprocess.run([harness, *map(str, args)], input=data, capture_output=True, check=True)
    return result.stdout, [int(line) for line in result.stderr.split()]


def hex32(value):
    """`value` as a Rust literal: 0x1234_abcd."""
    return f"0x{value >> 16:04x}_{value & 0xFFFF:04x}"


def number(value):
    """`value` as a Rust literal, its thousands separated."""
    return f"{value:_}"


def one_shot_table(harness):
    corpus = CORPUS.read_bytes()
    print("/// The zlib streams of `corpus.bin`: level, strategy, memory level,")
    print("/// length and CRC-32.")
    print("const CORPUS: [(u8, Strategy, u8, usize, u32); 70] = [")
    for memory in (8, 1, 9):
        for level in range(10):
            for name, code in STRATEGIES.items():
                if memory != 8 and name != "Default":
                    continue
                stream, _ = run(harness, ["oneshot", level, memory, code, 15], corpus)
                assert zlib.decompress(stream) == corpus
                print(
                    f"    ({level}, Strategy::{name}, {memory}, "
                    f"{number(len(stream))}, {hex32(zlib.crc32(stream))}),"
                )
    print("];")


def pixel(kind, x, y, c):
    """The image formulas of tests/zlib_ng.rs, integers only."""
    if kind == 0:
        return (x * 3 + y * 5 + c * 50 + x * y // 7) & 255
    if kind == 1:
        return ((x // 23 + y // 17) % 5 * 40 + c * 10) & 255
    noise = (x * 7919 + y * 104_729 + c * 31) * 2_654_435_761 >> 13 & 7
    return ((x * x + 3 * y * y) // 97 + 40 * c + noise) & 255


IMAGES = [
    ("RGBA", 300, 200, 0),
    ("RGB", 1000, 700, 2),
    ("L", 17_000, 3, 1),
    ("LA", 64, 48, 2),
]


def idat_rows(image):
    """The filtered rows of Pillow's PNG of `image`."""
    buffer = io.BytesIO()
    image.save(buffer, format="png")
    png, data, at = buffer.getvalue(), b"", 8
    while at < len(png):
        (length,) = struct.unpack(">I", png[at:at + 4])
        if png[at + 4:at + 8] == b"IDAT":
            data += png[at + 8:at + 8 + length]
        at += 12 + length
    return zlib.decompress(data)


def png_table(harness):
    print("/// The IDAT streams of the images, as Pillow writes them with")
    print("/// zlib-ng: image, level, strategy, length, CRC-32 and chunks.")
    print("const PNGS: [(usize, u8, Strategy, usize, u32, usize); 16] = [")
    for index, (mode, width, height, kind) in enumerate(IMAGES):
        channels = len(mode)
        data = bytes(
            pixel(kind, x, y, c) for y in range(height) for x in range(width) for c in range(channels)
        )
        rows = idat_rows(Image.frombytes(mode, (width, height), data))
        buffer = max(65_536, 4 * width)
        for level, name in ((1, "Filtered"), (6, "Filtered"), (9, "Filtered"), (9, "Default")):
            args = ["pillow", level, 9, STRATEGIES[name], 15, width * channels + 1, buffer]
            stream, chunks = run(harness, args, rows)
            assert zlib.decompress(stream) == rows
            print(
                f"    ({index}, {level}, Strategy::{name}, {number(len(stream))}, "
                f"{hex32(zlib.crc32(stream))}, {len(chunks)}),"
            )
    print("];")


if __name__ == "__main__":
    one_shot_table(sys.argv[1])
    print()
    png_table(sys.argv[1])
