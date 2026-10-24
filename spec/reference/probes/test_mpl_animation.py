"""matplotlib's own tests of ``matplotlib.animation``
(``matplotlib/tests/test_animation.py``): saving with the writers that need
no external program, the size of the frames, transparent frames, the embed
limit of the JavaScript player and the notebook representation.

Area ``mpl_animation``. Every probe runs in the style matplotlib's test
fixture applies (``classic`` plus ``_classic_test_patch``), given as
``style``. Unless noted, an op animates the tests' ``anim`` fixture:
``fig.subplots()`` of ``Figure()`` with an empty line, limits ``[0, 10] x
[-1, 1]``, an ``init_func`` emptying the line and a frame function setting
it to ``sin(x + i)`` over ``linspace(0, 10, 100)``, as
``FuncAnimation(fig, animate, init_func=init, frames=frames,
repeat=repeat)``.

Ops
---
``Animation.save``  {format, frames, repeat} -> the animation saved to
    ``movie.<format>`` with the writer ``test_save_animation_smoketest``
    uses for that file: for ``"gif"`` and ``"png"`` (an animated PNG) the
    ``"pillow"`` writer, recorded as Pillow reads the file: ``{frames, size,
    durations, loop}`` (the frame count, the size, each frame's duration in
    milliseconds and the loop count, null without one); for ``"html"`` the
    ``"html"`` writer, recorded as the names of the frame files it writes to
    ``movie_frames``. Or the error, for a GIF or PNG without frames.
    (``test_save_animation_smoketest``, the frame count of
    ``test_null_movie_writer``, the ``repeat=False`` saves of
    ``test_exhausted_animation`` and ``test_animation_frame``)
``Animation.save.frame_size``  {figsize, dpi} -> the size of the frames of
    ``FuncAnimation(fig, lambda frame: (), frames=1)`` on ``Figure(figsize,
    dpi)`` saved as a GIF (``test_frame_size``).
``Animation.save.invalid_path``  {format} -> the animation saved to
    ``missing/movie.<format>`` in an empty directory, with the writer of
    ``Animation.save``: the error (``test_movie_writer_invalid_path``; the
    writers check the directory before writing anything).
``Animation.save.transparent``  {} -> the smallest alpha of the first frame
    of the animation saved as a GIF with rcParams ``savefig.transparent``
    set, as Pillow reads it in RGBA (``test_animation_with_transparency``,
    whose ``grab_frame(transparent=True)`` is that rcParam's keyword).
``Animation.to_jshtml.embed_limit``  {frames, embed_limit} -> how many
    frames ``to_jshtml()`` embeds with rcParams ``animation.embed_limit``
    set (in megabytes) (``test_embed_limit``).
``Animation._repr_html_``  {html} -> with rcParams ``animation.html`` set:
    null when the representation is ``None``, else whether it contains
    ``"<script "`` (``test_animation_repr_html``; ``"html5"`` needs ffmpeg).

``test_animation_frame`` is the figure case ``mpl_rest/animation_frame``.

Left out: the ffmpeg and ImageMagick writers (external programs:
``test_failing_ffmpeg``, ``test_movie_writer_registry``, the ffmpeg and
ImageMagick parameters of ``test_save_animation_smoketest``), the writer
registry and writer objects plt-rs does not have (it picks the writer from
the file name: ``test_invalid_writer``, ``test_null_movie_writer``'s
custom ``AbstractMovieWriter``, ``test_movie_writer_dpi_default``,
``test_grabframe``), Python warnings and object lifetimes
(``test_animation_delete``, ``test_funcanimation_cache_frame_data``,
``test_no_frame_warning``, the warnings of ``test_exhausted_animation``,
``test_save_count_override_warnings_has_length``,
``test_save_count_override_warnings_scaler``, ``test_disable_cache_warning``),
frames from Python iterators (plt-rs animations count their frames:
``test_no_length_frames``), and ``test_draw_frame`` (Rust's types keep a
frame function from returning anything but artists).
"""

import pathlib
import tempfile

import matplotlib as mpl
import matplotlib.style
import numpy as np
from matplotlib import animation
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from PIL import Image

from spec_support import ProbeSet

probes = ProbeSet("mpl_animation")

CLASSIC = ["classic", "_classic_test_patch"]


def probe(op, fn, **args):
    def styled(style, **kw):
        with matplotlib.style.context(style):
            return fn(**kw)

    return probes.probe(op, styled, style=CLASSIC, **args)


def _anim(frames=5, repeat=True):
    """The ``anim`` fixture of ``test_animation.py``."""
    fig = Figure()
    FigureCanvasAgg(fig)
    ax = fig.subplots()
    line, = ax.plot([], [])
    ax.set_xlim(0, 10)
    ax.set_ylim(-1, 1)

    def init():
        line.set_data([], [])
        return line,

    def animate(i):
        x = np.linspace(0, 10, 100)
        line.set_data(x, np.sin(x + i))
        return line,

    return animation.FuncAnimation(fig=fig, func=animate, init_func=init, frames=frames,
                                   repeat=repeat)


WRITERS = {"gif": "pillow", "png": "pillow", "html": "html"}


def _read_movie(path):
    with Image.open(path) as im:
        durations = []
        for index in range(im.n_frames):
            im.seek(index)
            durations.append(im.info["duration"])
        return {"frames": im.n_frames, "size": list(im.size), "durations": durations,
                "loop": im.info.get("loop")}


def _save(format, frames, repeat):
    anim = _anim(frames, repeat)
    with tempfile.TemporaryDirectory() as tmp:
        path = pathlib.Path(tmp) / f"movie.{format}"
        anim.save(path, writer=WRITERS[format])
        if format == "html":
            return sorted(p.name for p in (pathlib.Path(tmp) / "movie_frames").iterdir())
        return _read_movie(path)


def test_save_animation_smoketest():
    for format in ["gif", "png", "html"]:
        for frames, repeat in [(5, True), (3, False)]:
            probe("Animation.save", _save, format=format, frames=frames, repeat=repeat)
    for format in ["gif", "png"]:
        probe("Animation.save", _save, format=format, frames=0, repeat=True)


def _frame_size(figsize, dpi):
    fig = Figure(figsize=figsize, dpi=dpi)
    FigureCanvasAgg(fig)
    anim = animation.FuncAnimation(fig, lambda frame: tuple(), frames=1)
    with tempfile.TemporaryDirectory() as tmp:
        path = pathlib.Path(tmp) / "movie.gif"
        anim.save(path, writer="pillow")
        with Image.open(path) as im:
            return list(im.size)


def test_frame_size():
    probe("Animation.save.frame_size", _frame_size, figsize=[1, 2.03], dpi=100)


def _invalid_path(format):
    anim = _anim()
    with tempfile.TemporaryDirectory() as tmp:
        anim.save(pathlib.Path(tmp) / "missing" / f"movie.{format}", writer=WRITERS[format])


def test_movie_writer_invalid_path():
    for format in ["gif", "html"]:
        probe("Animation.save.invalid_path", _invalid_path, format=format)


def _transparent():
    anim = _anim()
    with mpl.rc_context({"savefig.transparent": True}), tempfile.TemporaryDirectory() as tmp:
        path = pathlib.Path(tmp) / "movie.gif"
        anim.save(path, writer="pillow")
        with Image.open(path) as im:
            return im.convert("RGBA").getextrema()[3][0]


def test_animation_with_transparency():
    probe("Animation.save.transparent", _transparent)


def _embed_limit(frames, embed_limit):
    anim = _anim(frames)
    with mpl.rc_context({"animation.embed_limit": embed_limit}):
        return anim.to_jshtml().count("data:image/png;base64,")


def test_embed_limit():
    for embed_limit in [1e-6, 20.0]:
        probe("Animation.to_jshtml.embed_limit", _embed_limit, frames=1, embed_limit=embed_limit)


def _repr_html(html):
    anim = _anim()
    with mpl.rc_context({"animation.html": html}):
        representation = anim._repr_html_()
    return None if representation is None else "<script " in representation


def test_animation_repr_html():
    for html in ["none", "jshtml"]:
        probe("Animation._repr_html_", _repr_html, html=html)
