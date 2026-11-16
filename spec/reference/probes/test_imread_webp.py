"""Reading WebP files (``matplotlib.image.imread``).

Area ``imread_webp``. matplotlib opens WebP files with Pillow, which
decodes them with libwebp's ``WebPAnimDecoder``: the first frame on its
canvas, RGB, or RGBA when the file declares alpha. Each probe carries a
file made here deterministically from formula and ``spec_support.rng``
pixels: written by Pillow, by the libwebp Pillow bundles (called through
ctypes, for encoder settings Pillow does not expose: the simple loop
filter, several token partitions, segments, sharpness, alpha compression
and filters, near-lossless), or assembled from such files (animation
frames placed off the canvas origin).

Argument conventions
--------------------
As in area ``imread``: ``data`` is the file in base64, ``format`` is
``"webp"`` and ``case`` describes the file.

Ops
---
``image.imread``  {data, format, case} -> nested lists of ints
    ``(M, N, 3)`` for RGB and ``(M, N, 4)`` for RGBA; errors are
    ``{"error": "<ExceptionType>"}``.
"""

import ctypes
import glob
import os
import struct

import numpy as np
import PIL
import pytest
from PIL import Image

from spec_support import ProbeSet, rng
from spec_support.images import noise, pillow_bytes, probe_imread, ramp_rgb

probes = ProbeSet("imread_webp")


def _probe(case, data):
    return probe_imread(probes, "webp", case, data)


# --------------------------------------------------------------------------
# Pixels

def _rgba(width, height, seed=3):
    """Colour ramps under an alpha that sweeps from clear to opaque, with
    fully transparent and fully opaque patches."""
    rgb = ramp_rgb(width, height, seed)
    y, x = np.mgrid[0:height, 0:width]
    alpha = np.clip(255 * (x + y) / max(width + height - 2, 1), 0, 255)
    alpha[: height // 4, : width // 4] = 0
    alpha[-(height // 4):, -(width // 4):] = 255
    return np.dstack([rgb, alpha.astype(np.uint8)])


def _palette(width, height, colours, seed):
    """An image of `colours` distinct colours in blocks."""
    table = noise(colours, 1, 3, seed)[0]
    y, x = np.mgrid[0:height, 0:width]
    return table[((x // 3) + 2 * (y // 2)) % colours]


def _edges(width, height):
    """Sharp shapes: stripes, a disc and a diagonal, like plot lines."""
    y, x = np.mgrid[0:height, 0:width]
    image = np.full((height, width, 3), 250, np.uint8)
    image[(x // 4) % 2 == 0] = (30, 120, 220)
    disc = (x - width / 2) ** 2 + (y - height / 2) ** 2 < (min(width, height) / 3) ** 2
    image[disc] = (230, 90, 40)
    image[np.abs(x - y) < 2] = (0, 0, 0)
    return image


def _webp(array, **options):
    return pillow_bytes(Image.fromarray(array), "WEBP", **options)


# --------------------------------------------------------------------------
# libwebp through ctypes

_ABI = 0x0210


class _Config(ctypes.Structure):
    _fields_ = [(name, ctypes.c_float if name in ("quality", "target_PSNR") else ctypes.c_int)
                for name in ("lossless", "quality", "method", "image_hint", "target_size",
                             "target_PSNR", "segments", "sns_strength", "filter_strength",
                             "filter_sharpness", "filter_type", "autofilter",
                             "alpha_compression", "alpha_filtering", "alpha_quality", "pass_",
                             "show_compressed", "preprocessing", "partitions",
                             "partition_limit", "emulate_jpeg_size", "thread_level",
                             "low_memory", "near_lossless", "exact", "use_delta_palette",
                             "use_sharp_yuv", "qmin", "qmax")]


class _Picture(ctypes.Structure):
    _fields_ = [
        ("use_argb", ctypes.c_int), ("colorspace", ctypes.c_int),
        ("width", ctypes.c_int), ("height", ctypes.c_int),
        ("y", ctypes.c_void_p), ("u", ctypes.c_void_p), ("v", ctypes.c_void_p),
        ("y_stride", ctypes.c_int), ("uv_stride", ctypes.c_int),
        ("a", ctypes.c_void_p), ("a_stride", ctypes.c_int), ("pad1", ctypes.c_uint32 * 2),
        ("argb", ctypes.c_void_p), ("argb_stride", ctypes.c_int),
        ("pad2", ctypes.c_uint32 * 3),
        ("writer", ctypes.c_void_p), ("custom_ptr", ctypes.c_void_p),
        ("extra_info_type", ctypes.c_int), ("extra_info", ctypes.c_void_p),
        ("stats", ctypes.c_void_p), ("error_code", ctypes.c_int),
        ("progress_hook", ctypes.c_void_p), ("user_data", ctypes.c_void_p),
        ("pad3", ctypes.c_uint32 * 3), ("pad4", ctypes.c_void_p), ("pad5", ctypes.c_void_p),
        ("pad6", ctypes.c_uint32 * 8), ("memory_", ctypes.c_void_p),
        ("memory_argb_", ctypes.c_void_p), ("pad7", ctypes.c_void_p * 2),
    ]


class _MemoryWriter(ctypes.Structure):
    _fields_ = [("mem", ctypes.c_void_p), ("size", ctypes.c_size_t),
                ("max_size", ctypes.c_size_t), ("pad", ctypes.c_uint32 * 1)]


#: Where Pillow's wheels keep the libwebp they bundle: ``pillow.libs`` beside
#: the package on Linux, ``PIL/.dylibs`` on macOS. Its Windows wheels link
#: libwebp into ``_webp.pyd``, which exports none of it, so the files of
#: this area cannot be made there.
_LIBWEBP_GLOBS = (
    os.path.join(os.path.dirname(PIL.__file__), os.pardir, "pillow.libs", "libwebp-*.so*"),
    os.path.join(os.path.dirname(PIL.__file__), ".dylibs", "libwebp.*.dylib"),
)


def _libwebp_library():
    found = [path for pattern in _LIBWEBP_GLOBS for path in sorted(glob.glob(pattern))]
    if not found:
        pytest.skip("Pillow bundles no libwebp library to call here (Windows' links it in)",
                    allow_module_level=True)
    library = ctypes.CDLL(found[0])
    library.WebPConfigInitInternal.argtypes = [ctypes.POINTER(_Config), ctypes.c_int,
                                               ctypes.c_float, ctypes.c_int]
    library.WebPPictureInitInternal.argtypes = [ctypes.POINTER(_Picture), ctypes.c_int]
    library.WebPPictureImportRGBA.argtypes = [ctypes.POINTER(_Picture), ctypes.c_char_p,
                                              ctypes.c_int]
    library.WebPEncode.argtypes = [ctypes.POINTER(_Config), ctypes.POINTER(_Picture)]
    library.WebPValidateConfig.argtypes = [ctypes.POINTER(_Config)]
    library.WebPMemoryWriterInit.argtypes = [ctypes.POINTER(_MemoryWriter)]
    library.WebPMemoryWriterClear.argtypes = [ctypes.POINTER(_MemoryWriter)]
    library.WebPPictureFree.argtypes = [ctypes.POINTER(_Picture)]
    return library


_LIBWEBP = _libwebp_library()


def _libwebp(array, **settings):
    """The file libwebp's `WebPEncode` writes for an RGBA (or RGB) array
    with the default configuration changed by `settings`."""
    if array.shape[2] == 3:
        array = np.dstack([array, np.full(array.shape[:2], 255, np.uint8)])
    config = _Config()
    assert _LIBWEBP.WebPConfigInitInternal(ctypes.byref(config), 0, 75.0, _ABI)
    for name, value in settings.items():
        setattr(config, name, value)
    assert _LIBWEBP.WebPValidateConfig(ctypes.byref(config)), settings
    picture = _Picture()
    assert _LIBWEBP.WebPPictureInitInternal(ctypes.byref(picture), _ABI)
    height, width, _ = array.shape
    picture.width, picture.height, picture.use_argb = width, height, config.lossless
    pixels = np.ascontiguousarray(array).tobytes()
    assert _LIBWEBP.WebPPictureImportRGBA(ctypes.byref(picture), pixels, 4 * width)
    writer = _MemoryWriter()
    _LIBWEBP.WebPMemoryWriterInit(ctypes.byref(writer))
    picture.writer = ctypes.cast(_LIBWEBP.WebPMemoryWrite, ctypes.c_void_p).value
    picture.custom_ptr = ctypes.addressof(writer)
    ok = _LIBWEBP.WebPEncode(ctypes.byref(config), ctypes.byref(picture))
    data = ctypes.string_at(writer.mem, writer.size)
    _LIBWEBP.WebPPictureFree(ctypes.byref(picture))
    _LIBWEBP.WebPMemoryWriterClear(ctypes.byref(writer))
    assert ok, picture.error_code
    return data


# --------------------------------------------------------------------------
# Containers

def _chunk(fourcc, payload):
    return fourcc + struct.pack("<I", len(payload)) + payload + b"\0" * (len(payload) % 2)


def _riff(*chunks):
    body = b"".join(chunks)
    return b"RIFF" + struct.pack("<I", len(body) + 4) + b"WEBP" + body


def _chunks(file):
    """The chunks of a RIFF file, by four-character code, in order."""
    body, chunks = file[12:], []
    while body:
        size = struct.unpack("<I", body[4:8])[0]
        chunks.append((body[:4], body[8:8 + size]))
        body = body[8 + size + size % 2:]
    return chunks


def _u24(value):
    return struct.pack("<I", value)[:3]


def _offset_animation(file, width, height, x, y, frame_width, frame_height, alpha):
    """An animation whose only frame is the image of `file`, placed at
    (`x`, `y`) on a `width × height` canvas."""
    image = b"".join(_chunk(fourcc, payload) for fourcc, payload in _chunks(file)
                     if fourcc in (b"ALPH", b"VP8 ", b"VP8L"))
    flags = 0x02 | (0x10 if alpha else 0)
    vp8x = bytes([flags, 0, 0, 0]) + _u24(width - 1) + _u24(height - 1)
    anmf = (_u24(x // 2) + _u24(y // 2) + _u24(frame_width - 1) + _u24(frame_height - 1)
            + _u24(100) + b"\0" + image)
    return _riff(_chunk(b"VP8X", vp8x), _chunk(b"ANIM", bytes(6)), _chunk(b"ANMF", anmf))


# --------------------------------------------------------------------------
# Lossless

def test_lossless_pillow():
    rgb = ramp_rgb(33, 21, 30)
    _probe("lossless rgb", _webp(rgb, lossless=True))
    for quality in (0, 100):
        _probe(f"lossless rgb quality {quality}", _webp(rgb, lossless=True, quality=quality))
    for method in (0, 6):
        _probe(f"lossless rgb method {method}", _webp(rgb, lossless=True, method=method))
    rgba = _rgba(31, 17)
    _probe("lossless rgba", _webp(rgba, lossless=True))
    _probe("lossless rgba exact", _webp(rgba, lossless=True, exact=True))
    _probe("lossless noise", _webp(noise(16, 16, 3, 31), lossless=True))
    _probe("lossless rgba noise", _webp(noise(12, 10, 4, 32), lossless=True))
    _probe("lossless solid", _webp(np.full((5, 9, 3), (12, 200, 99), np.uint8), lossless=True))
    _probe("lossless grey", pillow_bytes(Image.fromarray(ramp_rgb(19, 11, 33)[:, :, 0]),
                                         "WEBP", lossless=True))


def test_lossless_palettes_and_sizes():
    for colours in (2, 4, 12, 200):
        _probe(f"lossless {colours} colours", _webp(_palette(29, 13, colours, 34 + colours),
                                                     lossless=True))
    for width, height in ((1, 1), (1, 13), (13, 1), (64, 3)):
        _probe(f"lossless {width}x{height}", _webp(ramp_rgb(width, height, 40), lossless=True))


def test_lossless_libwebp():
    rgb = ramp_rgb(40, 24, 41)
    _probe("lossless near-lossless 60", _libwebp(rgb, lossless=1, near_lossless=60))
    _probe("lossless near-lossless 0", _libwebp(rgb, lossless=1, near_lossless=0))
    _probe("lossless edges", _libwebp(_edges(48, 32), lossless=1, quality=100, method=6))
    _probe("lossless rgba method 0", _libwebp(_rgba(23, 19), lossless=1, method=0, quality=10))


# --------------------------------------------------------------------------
# Lossy

def test_lossy_pillow():
    rgb = ramp_rgb(48, 40, 50)
    _probe("lossy default", _webp(rgb))
    for quality in (0, 30, 100):
        _probe(f"lossy quality {quality}", _webp(rgb, quality=quality))
    for method in (0, 6):
        _probe(f"lossy method {method}", _webp(rgb, method=method))
    _probe("lossy noise", _webp(noise(32, 32, 3, 51), quality=95))
    _probe("lossy edges", _webp(_edges(64, 48), quality=90))
    _probe("lossy solid", _webp(np.full((32, 48, 3), (240, 240, 245), np.uint8)))


def test_lossy_sizes():
    for width, height in ((1, 1), (3, 5), (17, 9), (31, 33), (16, 16)):
        _probe(f"lossy {width}x{height}", _webp(ramp_rgb(width, height, 52)))


def test_lossy_alpha():
    rgba = _rgba(40, 24, 53)
    _probe("lossy rgba", _webp(rgba))
    for quality in (50, 0):
        _probe(f"lossy rgba alpha quality {quality}", _webp(rgba, alpha_quality=quality))
    _probe("lossy rgba odd", _webp(_rgba(13, 7, 54)))


def test_lossy_libwebp():
    rgb = ramp_rgb(64, 80, 60)
    _probe("lossy simple filter", _libwebp(rgb, filter_type=0, filter_strength=60))
    _probe("lossy no filter", _libwebp(rgb, filter_strength=0))
    for sharpness in (3, 7):
        _probe(f"lossy sharpness {sharpness}",
               _libwebp(rgb, filter_strength=80, filter_sharpness=sharpness))
    _probe("lossy autofilter", _libwebp(rgb, autofilter=1))
    for partitions in (1, 2, 3):
        _probe(f"lossy {1 << partitions} partitions", _libwebp(rgb, partitions=partitions))
    _probe("lossy one segment", _libwebp(rgb, segments=1))
    for sns in (0, 100):
        _probe(f"lossy sns {sns}", _libwebp(rgb, sns_strength=sns))
    _probe("lossy sharp yuv", _libwebp(rgb, use_sharp_yuv=1))
    _probe("lossy edges simple filter", _libwebp(_edges(48, 48), filter_type=0, quality=40))
    _probe("lossy quality 100", _libwebp(rgb, quality=100, method=6))


def test_lossy_alpha_libwebp():
    rgba = _rgba(37, 21, 61)
    _probe("lossy raw alpha", _libwebp(rgba, alpha_compression=0))
    for filtering in (0, 1, 2):
        _probe(f"lossy alpha filtering {filtering}", _libwebp(rgba, alpha_filtering=filtering))
    _probe("lossy alpha quality 30", _libwebp(rgba, alpha_quality=30))
    _probe("lossy raw alpha best filter", _libwebp(rgba, alpha_compression=0,
                                                   alpha_filtering=2))


# --------------------------------------------------------------------------
# Animations

def test_animations():
    frames = [Image.fromarray(ramp_rgb(24, 16, 70 + i)) for i in range(3)]
    animated = pillow_bytes(frames[0], "WEBP", save_all=True, append_images=frames[1:],
                            duration=100)
    _probe("animated lossy", animated)
    rgba = [Image.fromarray(_rgba(20, 14, 73 + i)) for i in range(2)]
    animated = pillow_bytes(rgba[0], "WEBP", save_all=True, append_images=rgba[1:],
                            duration=50, lossless=True)
    _probe("animated lossless rgba", animated)
    lossless = _webp(_rgba(9, 5, 75), lossless=True)
    _probe("offset lossless frame", _offset_animation(lossless, 16, 12, 4, 2, 9, 5, True))
    lossy = _webp(_rgba(9, 5, 76))
    _probe("offset lossy frame", _offset_animation(lossy, 13, 9, 2, 4, 9, 5, True))
    opaque = _webp(ramp_rgb(9, 5, 77))
    _probe("offset frame without alpha", _offset_animation(opaque, 12, 8, 2, 2, 9, 5, False))


# --------------------------------------------------------------------------
# Errors

def test_errors():
    lossless = _webp(ramp_rgb(17, 9, 80), lossless=True)
    _probe("truncated lossless", lossless[: len(lossless) // 2])
    lossy = _webp(ramp_rgb(17, 9, 81))
    _probe("truncated lossy", lossy[: len(lossy) - 7])
    _probe("not a bitstream", _riff(_chunk(b"VP8 ", bytes(range(40)))))
    _probe("bad lossless header", _riff(_chunk(b"VP8L", b"\x2e" + bytes(9))))
    corrupt = bytearray(lossless)
    corrupt[25:29] = rng(82).integers(4, 0, 256).astype(np.uint8).tobytes()
    _probe("corrupt lossless", bytes(corrupt))
