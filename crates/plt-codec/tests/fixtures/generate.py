"""Generate the PNG and zlib fixtures used by plt-codec's integration tests.

Run from anywhere with the reference environment:

    .venv/bin/python -I crates/plt-codec/tests/fixtures/generate.py

PNG fixtures are written by a minimal PNG writer below rather than by Pillow,
so that every colour type, bit depth and interlace combination (Pillow cannot
write 2/4-bit grey, 16-bit colour or interlaced files) and every scanline
filter type is covered. A few Pillow-written files check interoperability
with a real encoder. Each `<name>.png` comes with `<name>.expect`: the
de-interlaced raw samples in PNG layout, then the RGBA8 pixels, then the
RGBA16 pixels as little-endian u16. Expected RGBA values are cross-checked
against Pillow wherever its conversion semantics are unambiguous.

zlib fixtures are a mixed corpus and its compression by Python's zlib at
every level and with each strategy, window size and flush mode.
"""

from __future__ import annotations

import io
import pathlib
import struct
import zlib

import numpy as np
from PIL import Image, PngImagePlugin

HERE = pathlib.Path(__file__).resolve().parent
PNG_DIR = HERE / "png"
ZLIB_DIR = HERE / "zlib"

MASK64 = (1 << 64) - 1

# colour type -> (name, channels)
COLOR_TYPES = {0: ("gray", 1), 2: ("rgb", 3), 3: ("indexed", 1), 4: ("grayalpha", 2), 6: ("rgba", 4)}
ALLOWED_DEPTHS = {0: (1, 2, 4, 8, 16), 2: (8, 16), 3: (1, 2, 4, 8), 4: (8, 16), 6: (8, 16)}
ADAM7 = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)]


class Rng:
    """SplitMix64, as in the Rust tests."""

    def __init__(self, seed: int) -> None:
        self.state = seed & MASK64

    def next(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK64
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK64
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK64
        return z ^ (z >> 31)

    def below(self, bound: int) -> int:
        return (self.next() * bound) >> 64


# --- minimal PNG writer -----------------------------------------------------


def chunk(kind: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def pack_row(samples: np.ndarray, depth: int) -> bytes:
    """Pack one row of samples (flattened channels) at `depth` bits."""
    if depth == 16:
        return samples.astype(">u2").tobytes()
    if depth == 8:
        return samples.astype(np.uint8).tobytes()
    per_byte = 8 // depth
    padded = np.zeros(-(-len(samples) // per_byte) * per_byte, dtype=np.uint16)
    padded[: len(samples)] = samples
    out = bytearray()
    for group in padded.reshape(-1, per_byte):
        byte = 0
        for value in group:
            byte = (byte << depth) | int(value)
        out.append(byte)
    return bytes(out)


def paeth(a: int, b: int, c: int) -> int:
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    return b if pb <= pc else c


def filter_row(kind: int, row: bytes, prev: bytes, bpp: int) -> bytes:
    out = bytearray(len(row))
    for i, x in enumerate(row):
        a = row[i - bpp] if i >= bpp else 0
        b = prev[i]
        c = prev[i - bpp] if i >= bpp else 0
        predictor = [0, a, b, (a + b) // 2, paeth(a, b, c)][kind]
        out[i] = (x - predictor) & 0xFF
    return bytes([kind]) + bytes(out)


def filtered_stream(samples: np.ndarray, depth: int, channels: int) -> bytes:
    """Filter the rows of `samples` (height × width × channels), cycling the
    filter type through 0-4 row by row."""
    bpp = max(1, channels * depth // 8)
    rows = [pack_row(row.reshape(-1), depth) for row in samples]
    out = bytearray()
    prev = bytes(len(rows[0])) if rows else b""
    for y, row in enumerate(rows):
        out += filter_row(y % 5, row, prev, bpp)
        prev = row
    return bytes(out)


def image_stream(samples: np.ndarray, depth: int, channels: int, interlaced: bool) -> bytes:
    if not interlaced:
        return filtered_stream(samples, depth, channels)
    out = b""
    for x0, y0, dx, dy in ADAM7:
        sub = samples[y0::dy, x0::dx]
        if sub.shape[0] and sub.shape[1]:
            out += filtered_stream(sub, depth, channels)
    return out


def write_png(
    samples: np.ndarray,
    color_type: int,
    depth: int,
    interlaced: bool,
    *,
    palette: list[tuple[int, int, int]] | None = None,
    trns: bytes | None = None,
    extra_before: list[tuple[bytes, bytes]] = (),
    extra_after: list[tuple[bytes, bytes]] = (),
    idat_split: int | None = None,
) -> bytes:
    height, width, channels = samples.shape
    ihdr = struct.pack(">IIBBBBB", width, height, depth, color_type, 0, 0, int(interlaced))
    out = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
    for kind, data in extra_before:
        out += chunk(kind, data)
    if palette is not None:
        out += chunk(b"PLTE", bytes(v for entry in palette for v in entry))
    if trns is not None:
        out += chunk(b"tRNS", trns)
    compressed = zlib.compress(image_stream(samples, depth, channels, interlaced), 9)
    if idat_split:
        parts = [compressed[i : i + idat_split] for i in range(0, len(compressed), idat_split)]
        parts.insert(1, b"")
    else:
        parts = [compressed]
    for part in parts:
        out += chunk(b"IDAT", part)
    for kind, data in extra_after:
        out += chunk(kind, data)
    return out + chunk(b"IEND", b"")


# --- expected decoder output -------------------------------------------------


def raw_layout(samples: np.ndarray, depth: int) -> bytes:
    return b"".join(pack_row(row.reshape(-1), depth) for row in samples)


def expected_rgba16(samples, color_type, depth, palette=None, trns=None) -> np.ndarray:
    height, width, _ = samples.shape
    scale = 65535 // ((1 << depth) - 1)
    s = samples.astype(np.uint32)
    out = np.zeros((height, width, 4), dtype=np.uint32)
    if color_type == 3:
        table = np.array([(*rgb, 255) for rgb in palette], dtype=np.uint32)
        if trns is not None:
            table[: len(trns), 3] = list(trns)
        out[:] = table[s[..., 0]] * 257
        return out
    if color_type in (0, 4):
        out[..., 0:3] = s[..., 0:1] * scale
    else:
        out[..., 0:3] = s[..., 0:3] * scale
    if color_type in (4, 6):
        out[..., 3] = s[..., -1] * scale
    else:
        out[..., 3] = 65535
        if trns is not None:
            key = np.array(struct.unpack(f">{len(trns) // 2}H", trns), dtype=np.uint32)
            match = np.all(s == key, axis=-1)
            out[match, 3] = 0
    return out


def to_rgba8(rgba16: np.ndarray) -> np.ndarray:
    return ((rgba16 * 255 + 32895) >> 16).astype(np.uint8)


def write_fixture(name: str, png: bytes, raw: bytes, rgba16: np.ndarray, pillow_check: bool) -> None:
    rgba8 = to_rgba8(rgba16)
    if pillow_check:
        with Image.open(io.BytesIO(png)) as image:
            seen = np.asarray(image.convert("RGBA"))
        assert np.array_equal(seen, rgba8), f"{name}: Pillow disagrees"
    (PNG_DIR / f"{name}.png").write_bytes(png)
    expect = raw + rgba8.tobytes() + rgba16.astype("<u2").tobytes()
    (PNG_DIR / f"{name}.expect").write_bytes(expect)


def random_samples(rng: Rng, height: int, width: int, channels: int, limit: int) -> np.ndarray:
    """Smoothly varying samples with occasional jumps, in `0..limit`."""
    out = np.zeros((height, width, channels), dtype=np.int64)
    for y in range(height):
        for x in range(width):
            for c in range(channels):
                if rng.below(4) == 0:
                    out[y, x, c] = rng.below(limit)
                else:
                    base = out[y, x - 1, c] if x else (out[y - 1, x, c] if y else 0)
                    out[y, x, c] = (base + rng.below(5) - 2) % limit
    return out


def palette_for(rng: Rng, size: int) -> list[tuple[int, int, int]]:
    return [(rng.below(256), rng.below(256), rng.below(256)) for _ in range(size)]


# --- fixture sets --------------------------------------------------------------


def standard_formats(rng: Rng) -> None:
    """Every colour type / bit depth, plain and interlaced, 13×11 pixels."""
    palette_sizes = {1: 2, 2: 4, 4: 11, 8: 200}
    for color_type, (color_name, channels) in COLOR_TYPES.items():
        for depth in ALLOWED_DEPTHS[color_type]:
            for interlaced in (False, True):
                name = f"{color_name}{depth}" + ("_interlaced" if interlaced else "")
                palette = None
                limit = 1 << depth
                if color_type == 3:
                    limit = palette_sizes[depth]
                    palette = palette_for(rng, limit)
                samples = random_samples(rng, 11, 13, channels, limit)
                png = write_png(samples, color_type, depth, interlaced, palette=palette)
                rgba16 = expected_rgba16(samples, color_type, depth, palette)
                write_fixture(name, png, raw_layout(samples, depth), rgba16, pillow_check=depth <= 8)


def transparency(rng: Rng) -> None:
    cases = [
        ("gray8_trns", 0, 8, False, struct.pack(">H", 7)),
        ("gray2_trns_interlaced", 0, 2, True, struct.pack(">H", 1)),
        ("gray16_trns", 0, 16, False, struct.pack(">H", 300)),
        ("rgb8_trns", 2, 8, False, struct.pack(">HHH", 5, 6, 7)),
        ("rgb16_trns_interlaced", 2, 16, True, struct.pack(">HHH", 500, 600, 700)),
        ("indexed8_trns", 3, 8, False, bytes([0, 128, 255, 17])),
        ("indexed2_trns_interlaced", 3, 2, True, bytes([90])),
    ]
    for name, color_type, depth, interlaced, trns in cases:
        channels = COLOR_TYPES[color_type][1]
        palette = None
        limit = 1 << depth
        if color_type == 3:
            limit = min(limit, 6)
            palette = palette_for(rng, limit)
        samples = random_samples(rng, 11, 13, channels, min(limit, 9))
        if color_type in (0, 2):
            key = struct.unpack(f">{len(trns) // 2}H", trns)
            samples[0, 0], samples[5, 7], samples[10, 12] = key, key, key
        png = write_png(samples, color_type, depth, interlaced, palette=palette, trns=trns)
        rgba16 = expected_rgba16(samples, color_type, depth, palette, trns)
        # Pillow mishandles colour keys below 8 bits and at 16 bits.
        check = depth == 8
        write_fixture(name, png, raw_layout(samples, depth), rgba16, pillow_check=check)


def odd_sizes(rng: Rng) -> None:
    """Interlaced images with empty Adam7 passes."""
    for width, height, color_type, depth in [(1, 1, 6, 8), (9, 1, 0, 1), (1, 9, 2, 8), (2, 3, 0, 4), (5, 5, 4, 16)]:
        channels = COLOR_TYPES[color_type][1]
        samples = random_samples(rng, height, width, channels, 1 << depth)
        png = write_png(samples, color_type, depth, True)
        name = f"{COLOR_TYPES[color_type][0]}{depth}_{width}x{height}_interlaced"
        rgba16 = expected_rgba16(samples, color_type, depth)
        write_fixture(name, png, raw_layout(samples, depth), rgba16, pillow_check=depth <= 8)


def chunk_soup(rng: Rng) -> None:
    """Many IDAT chunks (one empty) amid ancillary chunks, known and private."""
    samples = random_samples(rng, 11, 13, 3, 256)
    before = [
        (b"gAMA", struct.pack(">I", 45455)),
        (b"sRGB", b"\x00"),
        (b"prVt", b"private ancillary data"),
        (b"pHYs", struct.pack(">IIB", 1000, 2000, 0)),
    ]
    after = [(b"tIME", struct.pack(">HBBBBB", 2026, 10, 16, 12, 0, 0)), (b"tEXt", b"Comment\x00after the data")]
    png = write_png(samples, 2, 8, False, extra_before=before, extra_after=after, idat_split=7)
    write_fixture("chunk_soup", png, raw_layout(samples, 8), expected_rgba16(samples, 2, 8), pillow_check=True)


def text_metadata(rng: Rng) -> None:
    samples = random_samples(rng, 3, 4, 4, 256)
    texts = [
        (b"tEXt", b"Software\x00Matplotlib version3.11.0, https://matplotlib.org/"),
        (b"zTXt", b"Comment\x00\x00" + zlib.compress("café crème".encode("latin-1"))),
        (b"iTXt", b"Title\x00\x00\x00en\x00Titel\x00" + "σ² plot".encode()),
        (b"iTXt", b"Description\x00\x01\x00\x00\x00" + zlib.compress("日本語".encode())),
        (b"pHYs", struct.pack(">IIB", 3937, 3937, 1)),
    ]
    png = write_png(samples, 6, 8, False, extra_before=texts)
    write_fixture("text_metadata", png, raw_layout(samples, 8), expected_rgba16(samples, 6, 8), pillow_check=True)


def pillow_written(rng: Rng) -> None:
    """Files written by Pillow, the encoder behind matplotlib's savefig."""
    rgba = random_samples(rng, 20, 30, 4, 256).astype(np.uint8)
    info = PngImagePlugin.PngInfo()
    info.add_text("Software", "Matplotlib version3.11.0, https://matplotlib.org/")
    buffer = io.BytesIO()
    Image.fromarray(rgba, "RGBA").save(buffer, "PNG", dpi=(72, 72), pnginfo=info)
    write_fixture("pillow_rgba", buffer.getvalue(), rgba.tobytes(), rgba.astype(np.uint32) * 257, pillow_check=True)

    gray16 = random_samples(rng, 9, 10, 1, 65536).astype(np.uint16)
    buffer = io.BytesIO()
    Image.fromarray(gray16[..., 0]).save(buffer, "PNG")
    write_fixture("pillow_gray16", buffer.getvalue(), raw_layout(gray16, 16), expected_rgba16(gray16, 0, 16), pillow_check=False)

    indices = random_samples(rng, 8, 12, 1, 16).astype(np.uint8)
    palette = palette_for(rng, 16)
    image = Image.fromarray(indices[..., 0], "P")
    image.putpalette([v for entry in palette for v in entry])
    buffer = io.BytesIO()
    image.save(buffer, "PNG", transparency=bytes([255] * 3 + [0] + [128] * 12), bits=4)
    png = buffer.getvalue()
    with Image.open(io.BytesIO(png)) as reread:
        assert reread.info.get("transparency") is not None
        written_palette = reread.getpalette()[: 3 * 16]
        bits = 4 if b"\x04\x03" in png[24:26] else 8
    rgb = [tuple(written_palette[i : i + 3]) for i in range(0, 48, 3)]
    alpha = bytes([255] * 3 + [0] + [128] * 12)
    rgba16 = expected_rgba16(indices, 3, bits, rgb, alpha)
    write_fixture("pillow_palette", png, raw_layout(indices, bits), rgba16, pillow_check=True)


def zlib_corpus() -> None:
    rng = Rng(2026)
    words = [b"plot ", b"axes ", b"figure ", b"tick ", b"label ", b"line ", b"marker ", b"legend\n"]
    parts = [
        b"".join(words[rng.below(len(words))] for _ in range(1500)),
        bytes(rng.below(256) for _ in range(3000)),
        bytes(4000),
        b"".join(bytes([rng.below(4)]) * rng.below(40) for _ in range(300)),
        bytes((i * i) % 251 for i in range(5000)),
    ]
    corpus = b"".join(parts)
    (ZLIB_DIR / "corpus.bin").write_bytes(corpus)
    streams = {f"level{level}": zlib.compress(corpus, level) for level in range(10)}

    def custom(level=6, wbits=15, strategy=zlib.Z_DEFAULT_STRATEGY, flush_every=None):
        compressor = zlib.compressobj(level, zlib.DEFLATED, wbits, 9, strategy)
        if flush_every is None:
            return compressor.compress(corpus) + compressor.flush()
        out = b""
        for i in range(0, len(corpus), flush_every):
            out += compressor.compress(corpus[i : i + flush_every])
            out += compressor.flush(zlib.Z_SYNC_FLUSH if (i // flush_every) % 2 else zlib.Z_FULL_FLUSH)
        return out + compressor.flush()

    streams["fixed"] = custom(strategy=zlib.Z_FIXED)
    streams["huffman_only"] = custom(strategy=zlib.Z_HUFFMAN_ONLY)
    streams["rle"] = custom(strategy=zlib.Z_RLE)
    streams["filtered"] = custom(strategy=zlib.Z_FILTERED)
    streams["window9"] = custom(wbits=9)
    streams["flushes"] = custom(flush_every=1000)
    for name, data in streams.items():
        assert zlib.decompress(data) == corpus
        (ZLIB_DIR / f"{name}.zlib").write_bytes(data)
    raw = custom(level=9, wbits=-15)
    (ZLIB_DIR / "level9_raw.deflate").write_bytes(raw)


def main() -> None:
    PNG_DIR.mkdir(exist_ok=True)
    ZLIB_DIR.mkdir(exist_ok=True)
    for old in [*PNG_DIR.iterdir(), *ZLIB_DIR.iterdir()]:
        old.unlink()
    rng = Rng(1)
    standard_formats(rng)
    transparency(rng)
    odd_sizes(rng)
    chunk_soup(rng)
    text_metadata(rng)
    pillow_written(rng)
    zlib_corpus()
    print(f"{len(list(PNG_DIR.glob('*.png')))} PNG fixtures, {len(list(ZLIB_DIR.iterdir()))} zlib fixtures")


if __name__ == "__main__":
    main()
