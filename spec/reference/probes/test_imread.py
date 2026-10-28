"""Reading JPEG, TIFF and GIF files (``matplotlib.image.imread``), and
rejecting truncated PNG files.

Area ``imread``. matplotlib opens every file that is not a PNG with
Pillow's ``Image.open`` and returns ``pil_to_array(image)``. Each probe
carries a file made here deterministically from formula and
``spec_support.rng`` pixels: written by Pillow, by this module's small
TIFF writer for the layouts Pillow cannot write (big-endian, tiles,
16-bit colour, associated alpha, fill order, orientation ...), or by
patching Pillow's output (JPEG sampling factors and tables, GIF colour
tables and screens).

Argument conventions
--------------------
* ``data``: the file's bytes in base64.
* ``format``: ``"jpeg"``, ``"tiff"`` or ``"gif"``, passed as imread's
  ``format``; any value but ``"png"`` makes matplotlib use
  ``Image.open``, which recognises the file by its contents. The
  truncated PNG files of ``test_png.py`` are read as ``"png"``, which
  both libraries reject.
* ``case``: a label describing the file (not passed to imread).

Ops
---
``image.imread``  {data, format, case} -> nested lists of ints
    ``(M, N)`` for greyscale (uint8 or uint16), ``(M, N, 3)`` for RGB and
    ``(M, N, 4)`` for RGBA; errors are ``{"error": "<ExceptionType>"}``.
"""

import io
import struct
import zlib

import matplotlib.pyplot as plt
import numpy as np
from PIL import Image

from spec_support import ProbeSet, rng
from spec_support.images import noise, pillow_bytes, probe_imread, ramp_rgb

probes = ProbeSet("imread")


def _gray(width, height, seed=2):
    return ramp_rgb(width, height, seed)[:, :, 1]


def _probe(format, case, data):
    return probe_imread(probes, format, case, data)


# --------------------------------------------------------------------------
# JPEG

def _jpeg(array, **options):
    """A JPEG of an (H, W) or (H, W, 3) uint8 array written by Pillow."""
    return pillow_bytes(Image.fromarray(array), "JPEG", **options)


def _cmyk(array, **options):
    """A CMYK JPEG of an (H, W, 4) uint8 array written by Pillow."""
    height, width, _ = array.shape
    return pillow_bytes(Image.frombytes("CMYK", (width, height), array.tobytes()), "JPEG", **options)


def _segments(data):
    """The (offset, marker, length) of each JPEG marker segment up to SOS."""
    found, at = [], 2
    while True:
        marker = data[at + 1]
        length = struct.unpack(">H", data[at + 2:at + 4])[0]
        found.append((at, marker, length))
        if marker == 0xDA:
            return found
        at += 2 + length


def _resample(data, factors, size):
    """A JPEG with the frame's sampling factors and size rewritten. With as
    many blocks per MCU and as many MCUs, the entropy-coded data stays a
    valid (other) image."""
    data = bytearray(data)
    for at, marker, _ in _segments(data):
        if marker in (0xC0, 0xC2):
            width, height = size
            data[at + 5:at + 9] = struct.pack(">HH", height, width)
            for c, factor in enumerate(factors):
                data[at + 11 + 3 * c] = factor
    return bytes(data)


def _wide_quant_tables(data):
    """A JPEG whose quantization tables are rewritten with 16-bit steps,
    large enough for the inverse DCT to overflow its 16-bit lanes."""
    steps = [255, 1000, 40000, 2, 65535, 300, 7, 32767]
    out = bytearray(data[:2])
    for at, marker, length in _segments(data):
        segment = data[at:at + 2 + length]
        if marker == 0xDB:
            payload, tables = segment[4:], bytearray()
            for start in range(0, len(payload), 65):
                tables.append(0x10 | (payload[start] & 15))
                for k in range(64):
                    tables += struct.pack(">H", steps[(k + start) % len(steps)])
            segment = b"\xff\xdb" + struct.pack(">H", len(tables) + 2) + bytes(tables)
        out += segment
    sos_at, _, sos_length = _segments(data)[-1]
    return bytes(out) + data[sos_at + 2 + sos_length:]


def _scan_offsets(data):
    """Offsets of each SOS marker and of the EOI of a JPEG."""
    found, at = [], 2
    while True:
        marker = data[at + 1]
        if marker == 0xD9:
            return found + [at]
        length = struct.unpack(">H", data[at + 2:at + 4])[0]
        if marker == 0xDA:
            found.append(at)
            at += 2 + length
            while not (data[at] == 0xFF and data[at + 1] not in (0, *range(0xD0, 0xD8))):
                at += 1
        else:
            at += 2 + length


def test_jpeg_baseline():
    for quality in (10, 50, 75, 95):
        for subsampling in ("4:4:4", "4:2:2", "4:2:0"):
            data = _jpeg(ramp_rgb(17, 9), quality=quality, subsampling=subsampling)
            _probe("jpeg", f"baseline q{quality} {subsampling} 17x9", data)
    for width, height in ((1, 1), (33, 2), (2, 33), (3, 5)):
        for subsampling in ("4:2:2", "4:2:0"):
            data = _jpeg(ramp_rgb(width, height, 3), subsampling=subsampling)
            _probe("jpeg", f"baseline {subsampling} {width}x{height}", data)
    _probe("jpeg", "baseline q85 4:2:0 260x6", _jpeg(ramp_rgb(260, 6, 4), quality=85))
    _probe("jpeg", "baseline q1 noise", _jpeg(noise(16, 16, 3, 5), quality=1, subsampling="4:4:4"))
    _probe("jpeg", "optimized tables", _jpeg(ramp_rgb(19, 11, 6), optimize=True))
    _probe("jpeg", "Adobe RGB", _jpeg(ramp_rgb(9, 7, 7), keep_rgb=True))


def test_jpeg_progressive():
    for subsampling in ("4:4:4", "4:2:2", "4:2:0"):
        data = _jpeg(ramp_rgb(33, 17, 8), progressive=True, subsampling=subsampling)
        _probe("jpeg", f"progressive {subsampling} 33x17", data)
    for width, height in ((1, 1), (17, 9), (33, 2)):
        data = _jpeg(ramp_rgb(width, height, 9), progressive=True, quality=90)
        _probe("jpeg", f"progressive q90 {width}x{height}", data)
    data = _jpeg(ramp_rgb(250, 5, 10), progressive=True, subsampling="4:2:2", optimize=True)
    _probe("jpeg", "progressive optimized 4:2:2 250x5", data)
    # Cut after a few scans, the image is block-smoothed by libjpeg.
    data = _jpeg(ramp_rgb(40, 24, 11), progressive=True, quality=85)
    offsets = _scan_offsets(data)
    for kept in (1, 2, 5):
        cut = data[:offsets[kept]] + b"\xff\xd9"
        _probe("jpeg", f"progressive cut after {kept} scans", cut)


def test_jpeg_restarts():
    for blocks in (1, 3):
        data = _jpeg(ramp_rgb(35, 18, 12), restart_marker_blocks=blocks)
        _probe("jpeg", f"restart every {blocks} MCUs", data)
        data = _jpeg(ramp_rgb(35, 18, 12), restart_marker_blocks=blocks, progressive=True)
        _probe("jpeg", f"progressive restart every {blocks} MCUs", data)
    data = _jpeg(ramp_rgb(35, 18, 12), restart_marker_rows=1, subsampling="4:4:4")
    _probe("jpeg", "restart every MCU row", data)


def test_jpeg_grey_and_cmyk():
    _probe("jpeg", "grey 17x9", _jpeg(_gray(17, 9)))
    _probe("jpeg", "grey progressive 20x13", _jpeg(_gray(20, 13), progressive=True))
    _probe("jpeg", "grey q5 noise", _jpeg(noise(13, 11, 1, 13)[:, :, 0], quality=5))
    _probe("jpeg", "grey 1x1", _jpeg(_gray(1, 1)))
    cmyk = np.dstack([ramp_rgb(9, 7, 14), noise(9, 7, 1, 15)])
    _probe("jpeg", "CMYK", _cmyk(cmyk, quality=80))
    _probe("jpeg", "CMYK progressive", _cmyk(cmyk, progressive=True))


def test_jpeg_sampling_and_tables():
    # 4:2:2 MCUs (two luma blocks across) become 4:4:0 ones (two down),
    # 4:2:0 ones (four luma blocks) 4:1:1 ones (four across).
    data = _jpeg(ramp_rgb(34, 18, 16), subsampling="4:2:2", quality=85)
    _probe("jpeg", "4:4:0", _resample(data, (0x12, 0x11, 0x11), (17, 36)))
    data = _jpeg(ramp_rgb(34, 18, 16), subsampling="4:2:0", quality=85)
    _probe("jpeg", "4:1:1", _resample(data, (0x41, 0x11, 0x11), (68, 9)))
    data = _jpeg(ramp_rgb(16, 8, 17), quality=100, subsampling="4:4:4")
    _probe("jpeg", "16-bit quantization tables", _wide_quant_tables(data))


def test_jpeg_errors():
    data = _jpeg(ramp_rgb(17, 9, 18))
    _probe("jpeg", "truncated", data[:len(data) // 2])
    _probe("jpeg", "not an image", b"\xff\xd8\xff" + bytes(range(40)))


# --------------------------------------------------------------------------
# TIFF

def _packbits(data):
    """PackBits: runs of 2 to 128 equal bytes, literals of up to 128."""
    out, at = bytearray(), 0
    while at < len(data):
        run = 1
        while at + run < len(data) and run < 128 and data[at + run] == data[at]:
            run += 1
        if run > 1:
            out += bytes([257 - run, data[at]])
            at += run
            continue
        start = at
        while at < len(data) and at - start < 128 and (
                at + 1 >= len(data) or data[at + 1] != data[at]):
            at += 1
        out += bytes([at - start - 1]) + data[start:at]
    return bytes(out)


def _lzw(data, old_style=False):
    """TIFF LZW: 9- to 12-bit codes, most significant bit first, widening a
    code early; old-style LZW packs them least significant bit first and
    widens them a code later."""
    out, buffer, count = bytearray(), 0, 0

    def put(code, width):
        nonlocal buffer, count
        if old_style:
            buffer |= code << count
            count += width
            while count >= 8:
                out.append(buffer & 255)
                buffer >>= 8
                count -= 8
        else:
            buffer = buffer << width | code
            count += width
            while count >= 8:
                count -= 8
                out.append(buffer >> count & 255)

    table, width, free = {bytes([i]): i for i in range(256)}, 9, 258
    put(256, width)
    current = b""
    for byte in data:
        extended = current + bytes([byte])
        if extended in table:
            current = extended
            continue
        put(table[current], width)
        table[extended] = free
        free += 1
        if free == 4094:
            put(256, width)
            table, width, free = {bytes([i]): i for i in range(256)}, 9, 258
        elif free == (1 << width) + old_style and width < 12:
            width += 1
        current = bytes([byte])
    if current:
        put(table[current], width)
    put(257, width)
    if count:
        out.append(buffer & 255 if old_style else buffer << (8 - count) & 255)
    return bytes(out)


def _difference(rows, samples, sample_bytes, order):
    """Horizontal differencing (predictor 2) of rows of samples."""
    dtype = np.dtype(f"{order}u{sample_bytes}")
    out = []
    for row in rows:
        values = np.frombuffer(row, dtype).astype(np.int64)
        diffs = values.copy()
        diffs[samples:] = values[samples:] - values[:-samples]
        out.append((diffs % (1 << (8 * sample_bytes))).astype(dtype).tobytes())
    return out


def _reverse_bits(data):
    """The bytes with their bits reversed (fill order 2)."""
    return bytes(int(f"{b:08b}"[::-1], 2) for b in data)


# The Compression tag value of each scheme the writer knows.
_COMPRESSIONS = {"raw": 1, "lzw": 5, "lzw-old": 5, "deflate": 8, "packbits": 32773}


def _compress(data, compression):
    return {
        "raw": lambda d: d,
        "lzw": _lzw,
        "lzw-old": lambda d: _lzw(d, old_style=True),
        "deflate": lambda d: zlib.compress(d, 9),
        "packbits": _packbits,
    }[compression](data)


def _tiff(rows, width, height, bits, photometric, *, order="<", compression="raw",
          extra=(), sample_format=None, predictor=None, rows_per_strip=None, tiles=None,
          colormap=None, fill_order=None, orientation=None, bigtiff=False):
    """A TIFF file of packed `rows` (bytes each) with the given tags."""
    samples = len(bits)
    bits_per_pixel = sum(bits)
    sample_bytes = bits[0] // 8 if bits[0] in (8, 16) else None

    def chunk_bytes(chunk_rows):
        if predictor == 2:
            chunk_rows = _difference(chunk_rows, samples, sample_bytes, order)
        data = _compress(b"".join(chunk_rows), compression)
        return _reverse_bits(data) if fill_order == 2 else data

    if tiles:
        tile_width, tile_length = tiles
        row_bytes = (tile_width * bits_per_pixel + 7) // 8
        chunks = []
        for top in range(0, height, tile_length):
            for left in range(0, width, tile_width):
                start = left * bits_per_pixel // 8
                tile_rows = [(rows[y][start:start + row_bytes] if y < height else b"")
                             .ljust(row_bytes, b"\0") for y in range(top, top + tile_length)]
                chunks.append(chunk_bytes(tile_rows))
    else:
        rows_per_strip = rows_per_strip or height
        chunks = [chunk_bytes(rows[top:top + rows_per_strip])
                  for top in range(0, height, rows_per_strip)]

    offset_type = 16 if bigtiff else 4
    tags = [
        (256, 4, [width]),
        (257, 4, [height]),
        (258, 3, list(bits)),
        (259, 3, [_COMPRESSIONS[compression]]),
        (262, 3, [photometric]),
    ]
    if fill_order:
        tags.append((266, 3, [fill_order]))
    if orientation:
        tags.append((274, 3, [orientation]))
    tags.append((277, 3, [samples]))
    if not tiles:
        tags.append((278, 4, [rows_per_strip]))
    if predictor:
        tags.append((317, 3, [predictor]))
    if colormap is not None:
        tags.append((320, 3, list(colormap)))
    if tiles:
        tags += [(322, 4, [tiles[0]]), (323, 4, [tiles[1]])]
    if extra:
        tags.append((338, 3, list(extra)))
    if sample_format:
        tags.append((339, 3, [sample_format] * samples))
    offsets_tag, counts_tag = (324, 325) if tiles else (273, 279)
    header = 16 if bigtiff else 8
    data, offsets = bytearray(), []
    for chunk in chunks:
        offsets.append(header + len(data))
        data += chunk
        if len(data) % 2:
            data += b"\0"
    counts = [len(chunk) for chunk in chunks]
    tags += [(offsets_tag, offset_type, offsets), (counts_tag, offset_type, counts)]
    tags.sort()

    formats = {3: "H", 4: "L", 16: "Q"}
    inline = 8 if bigtiff else 4
    directory_at = header + len(data)
    entry_size, count_format = (20, "Q") if bigtiff else (12, "H")
    values_at = directory_at + struct.calcsize(count_format) + entry_size * len(tags) + inline
    entries, values = bytearray(), bytearray()
    for tag, kind, items in tags:
        packed = struct.pack(f"{order}{len(items)}{formats[kind]}", *items)
        entries += struct.pack(f"{order}HH" + ("Q" if bigtiff else "L"), tag, kind, len(items))
        if len(packed) <= inline:
            entries += packed.ljust(inline, b"\0")
        else:
            entries += struct.pack(order + ("Q" if bigtiff else "L"), values_at + len(values))
            values += packed
    if bigtiff:
        head = b"II+\0" + struct.pack("<HHQ", 8, 0, directory_at)
    else:
        head = (b"II*\0" if order == "<" else b"MM\0*") + struct.pack(order + "L", directory_at)
    directory = struct.pack(order + count_format, len(tags)) + entries + b"\0" * inline
    return head + bytes(data) + directory + bytes(values)


def _rows(array, order="<"):
    """Byte rows of an (H, W, C) or (H, W) array of uint8 or uint16."""
    array = np.asarray(array)
    if array.dtype == np.uint16:
        array = array.astype(f"{order}u2")
    return [row.tobytes() for row in array]


def _bit_rows(levels, bits):
    """Byte rows of (H, W) levels packed `bits` to a sample."""
    rows = []
    for row in levels:
        value, count, out = 0, 0, bytearray()
        for level in row:
            value = value << bits | int(level)
            count += bits
            if count == 8:
                out.append(value)
                value, count = 0, 0
        if count:
            out.append(value << (8 - count))
        rows.append(bytes(out))
    return rows


def _colormap(entries, seed):
    """A ColorMap of `entries` colours: all reds, then greens, then blues."""
    values = rng(seed).integers(3 * entries, 0, 65536)
    return [int(v) for v in values]


def test_tiff_pillow_modes():
    rgb = ramp_rgb(13, 7, 20)
    rgba = np.dstack([rgb, noise(13, 7, 1, 21)])
    levels16 = (rgb[:, :, 0].astype(np.uint16) << 8) | rgb[:, :, 1]
    images = {
        "1": Image.fromarray(_gray(13, 7, 22) > 128),
        "L": Image.fromarray(_gray(13, 7, 23)),
        "LA": Image.merge("LA", [Image.fromarray(rgba[:, :, 1]), Image.fromarray(rgba[:, :, 3])]),
        "P": Image.fromarray(rgb).quantize(12, dither=Image.Dither.NONE),
        "RGB": Image.fromarray(rgb),
        "RGBA": Image.fromarray(rgba),
        "I;16": Image.fromarray(levels16),
        "I;16B": Image.fromarray(levels16.astype(">u2")),
        "CMYK": Image.frombytes("CMYK", (13, 7), rgba.tobytes()),
    }
    for mode, im in images.items():
        for compression in ("raw", "packbits", "tiff_lzw", "tiff_adobe_deflate"):
            try:
                data = pillow_bytes(im, "TIFF", compression=compression)
            except (ValueError, OSError):
                continue
            _probe("tiff", f"Pillow {mode} {compression}", data)
    im = Image.fromarray(ramp_rgb(21, 10, 24))
    for compression in ("tiff_lzw", "tiff_adobe_deflate"):
        data = pillow_bytes(im, "TIFF", compression=compression, tiffinfo={317: 2})
        _probe("tiff", f"Pillow RGB {compression} predictor", data)
    data = pillow_bytes(images["I;16"], "TIFF", compression="tiff_lzw", tiffinfo={317: 2})
    _probe("tiff", "Pillow I;16 LZW predictor", data)
    data = pillow_bytes(im, "TIFF", compression="tiff_lzw", strip_size=64)
    _probe("tiff", "Pillow RGB LZW small strips", data)


def test_tiff_layouts():
    rgb = ramp_rgb(11, 6, 30)
    alpha = noise(11, 6, 1, 31)
    rgba = np.dstack([rgb, alpha])
    wide = rgb.astype(np.uint16) * 257 + noise(11, 6, 3, 32)
    wide_alpha = np.dstack([wide, alpha.astype(np.uint16) * 256 + 7])
    premultiplied = np.dstack([(rgb.astype(np.uint16) * alpha // 255).astype(np.uint8), alpha])
    for order, name in (("<", "LE"), (">", "BE")):
        for compression in ("raw", "lzw", "deflate", "packbits"):
            data = _tiff(_rows(rgb), 11, 6, (8, 8, 8), 2, order=order, compression=compression,
                         rows_per_strip=4)
            _probe("tiff", f"RGB {name} {compression} strips of 4", data)
        data = _tiff(_rows(wide, order), 11, 6, (16,) * 3, 2, order=order)
        _probe("tiff", f"RGB 16-bit {name}", data)
        data = _tiff(_rows(wide_alpha, order), 11, 6, (16,) * 4, 2, order=order, extra=(2,),
                     compression="lzw", predictor=2)
        _probe("tiff", f"RGBA 16-bit {name} LZW predictor", data)
        data = _tiff(_rows(wide_alpha, order), 11, 6, (16,) * 4, 2, order=order, extra=(1,))
        _probe("tiff", f"RGBA 16-bit associated {name}", data)
        gray16 = wide[:, :, 0]
        data = _tiff(_rows(gray16, order), 11, 6, (16,), 1, order=order, compression="deflate",
                     predictor=2)
        _probe("tiff", f"grey 16-bit {name} deflate predictor", data)
        data = _tiff(_rows(rgba), 11, 6, (8,) * 4, 2, order=order, extra=(2,))
        _probe("tiff", f"RGBA {name}", data)
    data = _tiff(_rows(premultiplied), 11, 6, (8,) * 4, 2, extra=(1,), compression="packbits")
    _probe("tiff", "RGBA associated", data)
    data = _tiff(_rows(np.dstack([rgba, alpha])), 11, 6, (8,) * 5, 2, extra=(1, 0))
    _probe("tiff", "RGBA associated plus an extra sample", data)
    data = _tiff(_rows(rgba), 11, 6, (8,) * 4, 2, extra=(0,))
    _probe("tiff", "RGB plus an unspecified sample", data)
    data = _tiff(_rows(rgba), 11, 6, (8,) * 4, 2)
    _probe("tiff", "four samples without ExtraSamples", data)
    data = _tiff(_rows(np.dstack([rgb[:, :, 1], alpha[:, :, 0]])), 11, 6, (8, 8), 1, extra=(2,),
                 compression="lzw")
    _probe("tiff", "grey and alpha", data)
    data = _tiff(_rows(np.dstack([rgb[:, :, 1], alpha[:, :, 0]])), 11, 6, (8, 8), 1, extra=(1,))
    _probe("tiff", "grey and associated alpha (refused)", data)
    cmyk = np.dstack([rgb, alpha])
    data = _tiff(_rows(cmyk), 11, 6, (8,) * 4, 5, compression="deflate")
    _probe("tiff", "CMYK", data)
    data = _tiff(_rows(wide_alpha, ">"), 11, 6, (16,) * 4, 5, order=">")
    _probe("tiff", "CMYK 16-bit BE", data)


def test_tiff_grey_bilevel_and_palettes():
    levels = _gray(13, 5, 40)
    for bits in (1, 2, 4):
        packed = _bit_rows(levels >> (8 - bits), bits)
        for photometric in (0, 1):
            data = _tiff(packed, 13, 5, (bits,), photometric, compression="packbits")
            _probe("tiff", f"grey {bits}-bit photometric {photometric}", data)
        colormap = _colormap(1 << bits, 41 + bits)
        data = _tiff(packed, 13, 5, (bits,), 3, colormap=colormap, compression="lzw")
        _probe("tiff", f"palette {bits}-bit", data)
    data = _tiff(_rows(levels), 13, 5, (8,), 0)
    _probe("tiff", "grey white is zero", data)
    data = _tiff(_rows(levels), 13, 5, (8,), 3, colormap=_colormap(256, 45), order=">")
    _probe("tiff", "palette 8-bit BE", data)
    data = _tiff(_rows(levels), 13, 5, (8,), 3, colormap=_colormap(16, 46))
    _probe("tiff", "palette 8-bit with 16 colours", data)
    indexed_alpha = np.dstack([levels >> 4, noise(13, 5, 1, 47)[:, :, 0]])
    data = _tiff(_rows(indexed_alpha), 13, 5, (8, 8), 3, extra=(2,), colormap=_colormap(256, 48))
    _probe("tiff", "palette with alpha", data)
    bilevel = _bit_rows(levels > 120, 1)
    for compression in ("raw", "packbits", "lzw"):
        data = _tiff(bilevel, 13, 5, (1,), 0, compression=compression, fill_order=2)
        _probe("tiff", f"bilevel fill order 2 {compression}", data)
    data = _tiff(_rows(ramp_rgb(13, 5, 49)), 13, 5, (8, 8, 8), 2, fill_order=2)
    _probe("tiff", "RGB fill order 2", data)
    data = _tiff(_rows(levels), 13, 5, (8,), 1, sample_format=2)
    _probe("tiff", "signed 8-bit grey", data)


def test_tiff_tiles_and_structure():
    rgb = ramp_rgb(37, 21, 50)
    for compression in ("raw", "deflate", "lzw", "packbits"):
        data = _tiff(_rows(rgb), 37, 21, (8, 8, 8), 2, compression=compression, tiles=(16, 16),
                     predictor=2 if compression == "lzw" else None)
        _probe("tiff", f"RGB tiles {compression}", data)
    data = _tiff(_rows(rgb), 37, 21, (8, 8, 8), 2, tiles=(16, 32), order=">", compression="lzw")
    _probe("tiff", "RGB tiles BE", data)
    levels = _gray(37, 21, 51)
    data = _tiff(_bit_rows(levels > 100, 1), 37, 21, (1,), 1, tiles=(16, 16), compression="deflate")
    _probe("tiff", "bilevel tiles", data)
    data = _tiff(_rows(levels), 37, 21, (8,), 1, rows_per_strip=1, compression="lzw-old")
    _probe("tiff", "old-style LZW", data)
    small = ramp_rgb(9, 5, 52)
    for orientation in range(1, 9):
        data = _tiff(_rows(small), 9, 5, (8, 8, 8), 2, orientation=orientation)
        _probe("tiff", f"orientation {orientation}", data)
    data = _tiff(_rows(small), 9, 5, (8, 8, 8), 2, bigtiff=True, compression="deflate",
                 rows_per_strip=2)
    _probe("tiff", "BigTIFF", data)
    data = _tiff(_rows(small), 9, 5, (8, 8, 8), 2, compression="deflate")
    _probe("tiff", "truncated", data[:40])
    _probe("tiff", "not an image", b"II*\0" + bytes(range(30)))


# --------------------------------------------------------------------------
# GIF

def _gif_parts(data):
    """The screen descriptor, global colour table and blocks of a GIF."""
    flags, at = data[10], 13
    table = b""
    if flags & 0x80:
        table = data[13:13 + (3 << ((flags & 7) + 1))]
        at += len(table)
    blocks = []
    while data[at] != 0x3B:
        start = at
        if data[at] == 0x21:
            at += 2
        else:
            descriptor_flags = data[at + 9]
            at += 10
            if descriptor_flags & 0x80:
                at += 3 << ((descriptor_flags & 7) + 1)
            at += 1
        while data[at]:
            at += data[at] + 1
        at += 1
        blocks.append(data[start:at])
    return bytearray(data[:13]), table, blocks


def _gif(screen, table, blocks):
    """A GIF of a screen descriptor, global colour table and blocks."""
    return bytes(screen) + table + b"".join(blocks) + b";"


def _local_table(data):
    """The GIF with its global colour table moved into the image."""
    screen, table, blocks = _gif_parts(data)
    size_bits = screen[10] & 7
    screen[10] &= 0x7F
    out = []
    for block in blocks:
        if block[0] == 0x2C:
            block = bytearray(block)
            block[9] |= 0x80 | size_bits
            block = bytes(block[:10]) + table + bytes(block[10:])
        out.append(block)
    return _gif(screen, b"", out)


def _reframed(data, screen_size, origin):
    """The GIF with another screen size and the image moved to `origin`."""
    screen, table, blocks = _gif_parts(data)
    screen[6:10] = struct.pack("<HH", *screen_size)
    out = []
    for block in blocks:
        if block[0] == 0x2C:
            block = bytearray(block)
            block[1:5] = struct.pack("<HH", *origin)
        out.append(bytes(block))
    return _gif(screen, table, out)


def _palette_image(width, height, colours, seed):
    """A mode P image of random indices into `colours` random colours."""
    indices = rng(seed).integers(width * height, 0, colours).astype(np.uint8)
    im = Image.frombytes("P", (width, height), indices.tobytes())
    im.putpalette([int(v) for v in rng(seed + 1).integers(3 * colours, 0, 256)])
    return im


def test_gif():
    for colours in (2, 7, 200):
        im = _palette_image(17, 9, colours, 60 + colours)
        _probe("gif", f"{colours} colours", pillow_bytes(im, "GIF", optimize=False))
        _probe("gif", f"{colours} colours transparent", pillow_bytes(im, "GIF", transparency=1,
                                                               optimize=False))
    im = _palette_image(23, 21, 16, 70)
    _probe("gif", "interlaced", pillow_bytes(im, "GIF", interlace=True, optimize=False))
    _probe("gif", "not interlaced", pillow_bytes(im, "GIF", interlace=False, optimize=False))
    data = pillow_bytes(im, "GIF", interlace=True, transparency=3, optimize=False)
    _probe("gif", "interlaced transparent local table", _local_table(data))
    _probe("gif", "local table", _local_table(pillow_bytes(im, "GIF", optimize=False)))
    _probe("gif", "RGB quantized", pillow_bytes(Image.fromarray(ramp_rgb(19, 12, 71)), "GIF"))
    _probe("gif", "grey ramp", pillow_bytes(Image.fromarray(_gray(15, 6, 72)), "GIF", optimize=False))
    small = pillow_bytes(_palette_image(6, 4, 5, 73), "GIF", optimize=False)
    _probe("gif", "frame inside a larger screen", _reframed(small, (11, 9), (3, 2)))
    small_transparent = pillow_bytes(_palette_image(6, 4, 5, 73), "GIF", transparency=0,
                                optimize=False)
    _probe("gif", "transparent frame inside a larger screen",
           _reframed(small_transparent, (11, 9), (3, 2)))
    _probe("gif", "frame past the screen", _reframed(small, (4, 3), (1, 1)))
    _probe("gif", "truncated", small[:-6])
    table = 3 << ((small[10] & 7) + 1) if small[10] & 0x80 else 0
    _probe("gif", "no image", small[:13 + table] + b";")


# --------------------------------------------------------------------------
# Truncated PNG files (matplotlib's tests/test_png.py)

def _saved_png():
    """The PNG ``plt.savefig`` writes of an empty figure."""
    buffer = io.BytesIO()
    plt.savefig(buffer)
    plt.close("all")
    return buffer.getvalue()


def test_truncated_file():
    _probe("png", "first 20 bytes of a saved figure, from a file", _saved_png()[:20])


def test_truncated_buffer():
    _probe("png", "first 20 bytes of a saved figure, from a buffer", _saved_png()[:20])
