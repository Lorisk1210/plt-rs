"""matplotlib's own non-image tests of ``matplotlib.image``
(``tests/test_image.py``) that the ``image_behaviour``, ``imread`` and
``imsave`` areas leave out: the cursor data of unnormalised bytes, the
interpolation stage from rcParams, RGB data clipped into range, round trips
through ``imsave`` and ``imread``, the metadata ``imsave`` writes, a JPEG of
a transparent figure image, and resampling through affine and non-affine
transforms.

Area ``mpl_image``.

Ops
---
``image.AxesImage.format_cursor_data``  {data, dtype, norm, point} ->
    ``im.format_cursor_data(im.get_cursor_data(event))`` for a mouse event
    at data ``point`` over ``im = ax.imshow(np.array(data, dtype),
    norm=norm)`` (``norm`` is ``null`` or ``"NoNorm"``), the figure drawn.
``image.AxesImage.get_interpolation_stage``  {rc} -> the stage of
    ``ax.imshow([[1, 2]])`` under ``image.interpolation_stage = rc``.
``rc.validate``  {key, value} -> the parameter's canonical value, or the
    error (the ``rcsetup`` area's).
``image.AxesImage.get_array``  {dtype} -> ``ax.imshow(arr).get_array()``
    for ``arr = (np.arange(300, dtype=dtype).reshape((10, 10, 3)) - 10) /
    255``: RGB colours below 0 and above 1, clipped.
``image.imsave.imread``  {data, format, dpi?, origin?} -> ``{"shape", "sum",
    "weighted"}`` of ``imread`` of what ``imsave`` writes of ``data``: the
    shape read back, and the sum and the position-weighted sum (``sum((i +
    1) * v)`` over the flattened array) of its values as bytes (floats
    times 255, rounded). ``data`` is ``{"uniform": seed, "shape"}``
    (``spec_support.rng(seed).uniform``) or ``{"zeros": shape}``.
``image.imsave.png_text``  {format, metadata?} -> {chunks, text}: the chunk
    types before the image data of ``imsave(buffer, [[0, 1], [2, 3]],
    format=format, metadata=metadata)`` (no ``metadata`` when absent; a
    ``null`` value drops the key) and the text entries Pillow reads from it,
    in file order: matplotlib's default ``Software`` entry (recorded as
    ``<default software>``, since each library names itself there), keys
    PNG defines, custom ones and text outside Latin-1, as the ``mpl_agg``
    area's ``Figure.savefig.png_text`` records them. Other formats refuse
    metadata (``test_imsave_pil_kwargs_png``, through ``metadata`` rather
    than Pillow's ``pnginfo``).
``pyplot.savefig.jpg``  {} -> ``{"num_colors", "corner"}`` of
    ``test_jpeg_alpha``'s JPEG: a 1 in figure at 300 dpi showing a black
    ``figimage`` whose alpha ramps from 0 to 1 left to right, saved with a
    red face colour; the number of colours in it and its top left pixel.
``image.resample``  {data, interpolation, width, nonaffine} -> the row
    ``mimage.resample`` produces from the row ``data`` into a zeroed row
    of ``width`` pixels through ``Affine2D().scale(width / len(data), 1)``,
    behind the test's non-affine identity when ``nonaffine``.

Cases
-----
Named after the upstream tests; random data comes from ``spec_support.rng``
with the tests' seeds. The cursor probe draws its figure first: until
then matplotlib has not applied the image's equal aspect, while pltrs
lays a figure out before reading it.

Already covered
---------------
By the ``image_behaviour`` area: ``test_cursor_data``,
``test_cursor_data_nonuniform``, ``test_format_cursor_data``,
``test_get_window_extent_for_AxisImage``, ``test_mask_image_all``,
``test_imshow_no_warn_invalid``, ``test_full_invalid``, ``test_composite``,
``test_relim``, ``test_exact_vmin`` and
``test_non_transdata_image_does_not_touch_aspect``. By the ``imread`` area:
``test_imread_pil_uint16`` (16-bit TIFFs). By the ``images`` figure group:
``test_imshow_bool``. By the ``mpl_image`` figure group: every image test
and the ``check_figures_equal`` tests. By the ``norms`` area: the unknown
norm name of ``test_str_norms``.

Left out
--------
Python and numpy mechanics pltrs's types rule out: ``test_image_python_io``
(``BytesIO``), ``test_imread_fspath``, ``test_imsave_fspath``,
``test_imsave_python_sequences``, ``test_imsave_rgba_origin`` (array
contiguity), ``test_imsave_pil_kwargs_tiff`` (Pillow options), ``test_imshow_pil``
and ``test_jpeg_2d`` (PIL images), ``test_imshow_multi_draw``,
``test_axesimage_setdata``, ``test_figureimage_setdata`` and
``test_setdata_xya`` (numpy views and copies), ``test_imshow_10_10_2``,
``test_imshow_10_10_5``, ``test_image_array_alpha_validation``,
``test_axesimage_get_shape``, ``test_empty_imshow``,
``test_nonuniformimage_setcmap`` and ``test_nonuniformimage_setnorm``
(images without data or of shapes the image data types rule out),
``test_imshow_float16``, ``test_imshow_float128``, ``test_quantitynd``,
``test_imshow_quantitynd``, ``test_resample_dtypes`` and the integer dtypes
of ``test_imshow_clips_rgb_to_valid_range`` (numpy dtypes: pltrs's
integer colours are bytes), ``test_format_cursor_data_multinorm`` (private
attributes), ``test_load_from_url`` (URLs), ``test__resample_valid_output``
and ``test__resample_nonaffine_mesh_shape`` (argument checks of the Python
binding). ``test_image_edges``, ``test_unclipped``, ``test_respects_bbox``,
``test_image_preserve_size2``, ``test_minimized_rasterized`` and
``test_rgba_stage_demultiplies_resampled_alpha`` are the
``mpl_tests_image`` figure group's.
"""

import io

import matplotlib as mpl
import matplotlib.image as mimage
import matplotlib.pyplot as plt
import numpy as np
from matplotlib import colors
from matplotlib.backend_bases import MouseEvent
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.transforms import Affine2D, Transform
from PIL import Image

from probes.test_mpl_agg import _DEFAULT_SOFTWARE, _chunk_types
from probes.test_rcsetup import _validate
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_image")

NORMS = {None: None, "NoNorm": colors.NoNorm}


def _format_cursor_data(data, dtype, norm, point):
    fig = Figure()
    FigureCanvasAgg(fig)
    ax = fig.subplots()
    make_norm = NORMS[norm]
    im = ax.imshow(np.array(data, dtype=dtype), norm=make_norm and make_norm())
    fig.draw_without_rendering()
    xdisp, ydisp = ax.transData.transform(point)
    event = MouseEvent('motion_notify_event', fig.canvas, xdisp, ydisp)
    return im.format_cursor_data(im.get_cursor_data(event))


def test_format_cursor_data_uint8_no_norm():
    r = probes.probe("image.AxesImage.format_cursor_data", _format_cursor_data,
                     data=[[0, 44]], dtype="uint8", norm="NoNorm", point=[1, 0])
    assert r == "[44.000]"


def _interpolation_stage(rc):
    with mpl.rc_context({"image.interpolation_stage": rc}):
        return Figure().subplots().imshow([[1, 2]]).get_interpolation_stage()


def test_rc_interpolation_stage():
    for val in ["data", "rgba"]:
        assert probes.probe("image.AxesImage.get_interpolation_stage", _interpolation_stage,
                            rc=val) == val
    for val in ["DATA", "foo", "None"]:
        r = probes.probe("rc.validate", _validate, key="image.interpolation_stage", value=val)
        assert isinstance(r, ValueError)


def _get_array(dtype):
    arr = np.arange(300, dtype=dtype).reshape((10, 10, 3))
    arr -= 10
    return Figure().subplots().imshow(arr / 255).get_array()


def test_imshow_clips_rgb_to_valid_range():
    for dtype in ["float32", "float64"]:
        out = probes.probe("image.AxesImage.get_array", _get_array, dtype=dtype)
        assert out.min() == 0 and out.max() == 1


def _data(spec):
    if "zeros" in spec:
        return np.zeros(spec["zeros"])
    shape = spec["shape"]
    return rng(spec["uniform"]).uniform(int(np.prod(shape))).reshape(shape)


def _round_trip(data, format, dpi=None, origin=None):
    buff = io.BytesIO()
    kw = {key: value for key, value in [("dpi", dpi), ("origin", origin)] if value is not None}
    plt.imsave(buff, _data(data), format=format, **kw)
    buff.seek(0)
    arr = plt.imread(buff, format=format)
    values = np.round(arr * 255) if arr.dtype.kind == "f" else arr
    values = values.astype(np.int64).ravel()
    return {"shape": arr.shape, "sum": int(values.sum()),
            "weighted": int((values * np.arange(1, values.size + 1)).sum())}


def test_imsave():
    data = {"uniform": 1, "shape": [1856, 2]}
    for fmt in ["png", "jpg", "jpeg", "tiff"]:
        for dpi in [1, 100]:
            probes.probe("image.imsave.imread", _round_trip, data=data, format=fmt, dpi=dpi)


def test_imsave_color_alpha():
    for origin in ["lower", "upper"]:
        probes.probe("image.imsave.imread", _round_trip,
                     data={"uniform": 1, "shape": [16, 16, 4]}, format="png", origin=origin)


def test_image_preserve_size():
    r = probes.probe("image.imsave.imread", _round_trip, data={"zeros": [481, 321]},
                     format="png")
    assert list(r["shape"][:2]) == [481, 321]


def _imsave_text(format, metadata=None):
    buff = io.BytesIO()
    plt.imsave(buff, [[0, 1], [2, 3]], format=format, metadata=metadata)
    png = buff.getvalue()
    text = Image.open(io.BytesIO(png)).text.items()
    return {"chunks": _chunk_types(png),
            "text": [[key, "<default software>" if key == "Software" and value == _DEFAULT_SOFTWARE
                      else value] for key, value in text]}


def test_imsave_metadata():
    probes.probe("image.imsave.png_text", _imsave_text, format="png")
    for metadata in [{}, {"Software": None}, {"Software": "test"},
                     {"Title": "Ramp", "Software": "s"},
                     {"Title": "Ramp", "Author": "Zoë", "Description": "日本語の説明",
                      "Comment": "first line\nsecond line", "Custom key": "value", "Unset": None}]:
        probes.probe("image.imsave.png_text", _imsave_text, format="png", metadata=metadata)
    for format in ["jpeg", "tiff"]:
        r = probes.probe("image.imsave.png_text", _imsave_text, format=format, metadata={})
        assert isinstance(r, ValueError)


def _jpeg_alpha():
    fig = Figure(figsize=(1, 1), dpi=300)
    FigureCanvasAgg(fig)
    im = np.zeros((300, 300, 4), dtype=float)
    im[..., 3] = np.linspace(0.0, 1.0, 300)
    fig.figimage(im)
    buff = io.BytesIO()
    fig.savefig(buff, facecolor="red", format='jpg', dpi=300)
    buff.seek(0)
    image = Image.open(buff)
    return {"num_colors": len(image.getcolors(256)), "corner": image.getpixel((0, 0))}


def test_jpeg_alpha():
    r = probes.probe("pyplot.savefig.jpg", _jpeg_alpha)
    assert 175 <= r["num_colors"] <= 230 and tuple(r["corner"]) == (254, 0, 0)


class NonAffineIdentity(Transform):
    """The ``nonaffine_identity`` fixture."""

    input_dims = 2
    output_dims = 2

    def inverted(self):
        return self


def _resample(data, interpolation, width, nonaffine):
    data = np.array([data], dtype=float)
    transform = Affine2D().scale(sx=width / data.shape[1], sy=1)
    if nonaffine:
        transform = NonAffineIdentity() + transform
    out = np.zeros((1, width))
    mimage.resample(data, out, transform, interpolation=getattr(mimage, interpolation.upper()))
    return out[0]


def test_resample_nonaffine():
    for data, interpolation, width in [
            ([0.1, 0.3, 0.2], "nearest", 10),
            ([0.1, 0.2, 0.3, 0.4, 0.5, 0.6], "nearest", 9),
            ([0.1, 0.3, 0.2], "bilinear", 10),
            ([0.1, 0.9], "bilinear", 20),
            ([0.1, 0.1], "bilinear", 10),
            ([0.1, 0.9], "nearest", 1024),
            ([0.1, 0.9], "bilinear", 1024)]:
        for nonaffine in [False, True]:
            probes.probe("image.resample", _resample, data=data, interpolation=interpolation,
                         width=width, nonaffine=nonaffine)
