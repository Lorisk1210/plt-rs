"""
``benchmarks/compare``, the tool that runs a script in matplotlib's
syntax under matplotlib and under pltrs (Python) and compares the two:
what needs only pltrs (this venv has no matplotlib). The child's protocol
on tiny scripts (a time, a picture, the versions; a script without
``show``, one that only saves a file, one that raises), the alias that
makes the script's ``import matplotlib.pyplot`` pltrs, the clip's frames
before, between and after the two times, and the descriptions.
"""

import pathlib
import sys
import textwrap

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]
TOOL = ROOT / "benchmarks" / "compare"
if not (TOOL / "compare.py").is_file():
    pytest.skip("the comparison tool is in the source tree only", allow_module_level=True)
sys.path.insert(0, str(TOOL))

import child  # noqa: E402
import compare  # noqa: E402
import encode  # noqa: E402
import frames  # noqa: E402
import pictures  # noqa: E402
import runs  # noqa: E402
from PIL import Image  # noqa: E402

#: The colours of the two pictures of the frame tests.
MATPLOTLIB_RED = (220, 30, 30)
PLTRS_BLUE = (30, 30, 220)


@pytest.fixture(scope="module")
def pltrs_side():
    """pltrs' side as the tool makes it for this interpreter."""
    sources = None if compare.finds_pltrs(sys.executable) else str(compare.SOURCES)
    return runs.Side("pltrs", sys.executable, alias=str(compare.ALIAS), sources=sources)


def run_script(side, tmp_path, source, look="classic"):
    """Runs *source* once on *side*; returns the `runs.Run`."""
    tmp_path.mkdir(exist_ok=True)
    script = tmp_path / "script.py"
    script.write_text(textwrap.dedent(source), encoding="utf-8")
    return runs.run_once(side, script, look, tmp_path / "run", tmp_path / "config")


def test_the_spans_are_measured_on_one_clock(monkeypatch):
    # The system's clock steps by 16 ms on Windows before Python 3.13
    # (here: by a whole second), while the pictures are kept on the steady
    # clock: the span to the end, less that time, is never before the
    # figure was ready.
    import time

    monkeypatch.setattr(child.time, "time", lambda: 1_000.0)
    monkeypatch.setattr(child, "_open_figures", lambda: [])
    ready = child._Ready(999.75)
    time.sleep(0.01)
    ready.stamp()
    time.sleep(0.01)
    end = ready.elapsed() - ready.keeping
    assert 0.25 < ready.ready <= end < 1.25


def test_a_script_shown_is_timed_to_its_show_and_its_figure_kept(pltrs_side, tmp_path):
    run = run_script(pltrs_side, tmp_path, """
        import matplotlib.pyplot as plt
        fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
        ax.plot([0, 1], [0, 1])
        plt.show()
        print("after show")
    """)
    assert run.error is None
    assert 0 < run.ready <= run.end
    assert run.peak_memory is None or run.peak_memory > 0
    assert len(run.pictures) == 1
    with Image.open(run.pictures[0]) as picture:
        assert picture.size == (150, 100)
    assert run.version == compare_version()
    assert pathlib.Path(run.location).name == "pltrs"


def compare_version():
    """The workspace's version, which the checkout's pltrs reports."""
    import tomllib

    with open(ROOT / "Cargo.toml", "rb") as file:
        return tomllib.load(file)["workspace"]["package"]["version"]


def test_the_scripts_import_of_matplotlib_gets_pltrs(pltrs_side, tmp_path):
    run = run_script(pltrs_side, tmp_path, """
        import pathlib
        import matplotlib
        import matplotlib.pyplot as plt
        import pltrs.pyplot
        assert pltrs.pyplot is plt, (pltrs.pyplot, plt)
        assert pathlib.Path(matplotlib.__file__).parent.name == "pltrs", matplotlib.__file__
        plt.figure(figsize=(2, 2), dpi=10)
        plt.show()
    """)
    assert run.error is None, run.error


def test_the_look_of_pltrs_side_is_chosen_by_the_tool(pltrs_side, tmp_path):
    source = """
        import os, matplotlib.pyplot as plt
        assert os.environ["PLTRS_LOOK"] == {look!r}
        assert os.environ["MPLBACKEND"] == "agg"
        plt.figure(figsize=(1, 1), dpi=10)
    """
    for look, value in runs.LOOKS.items():
        run = run_script(pltrs_side, tmp_path / look, source.format(look=value), look=look)
        assert run.error is None, run.error


def test_a_script_without_show_is_timed_to_its_end(pltrs_side, tmp_path):
    run = run_script(pltrs_side, tmp_path, """
        import matplotlib.pyplot as plt
        plt.figure(figsize=(2, 1), dpi=20)
        plt.figure(figsize=(1, 1), dpi=20)
    """)
    assert run.error is None
    assert run.ready == pytest.approx(run.end, abs=0.05)
    assert len(run.pictures) == 2


def test_a_script_that_only_saves_shows_the_image_it_saved(pltrs_side, tmp_path):
    run = run_script(pltrs_side, tmp_path, """
        import matplotlib.pyplot as plt
        fig = plt.figure(figsize=(2, 1), dpi=20)
        fig.savefig("saved.png")
        plt.close(fig)
    """)
    assert run.error is None
    assert [pathlib.Path(path).name for path in run.pictures] == ["saved.png"]


def test_a_script_may_print_what_the_systems_code_page_lacks(pltrs_side, tmp_path):
    # The child's output is a pipe, encoded on Windows in its ANSI code
    # page (cp1252) unless the tool says otherwise.
    run = run_script(pltrs_side, tmp_path, """
        import matplotlib.pyplot as plt
        print("x \u2192 y \u2265 0, 5 \u00b5s")
        raise ValueError("\u2192 refused")
    """)
    assert "ValueError: \u2192 refused" in run.error


def test_a_script_that_raises_reports_its_traceback(pltrs_side, tmp_path):
    run = run_script(pltrs_side, tmp_path, """
        import matplotlib.pyplot as plt
        raise ValueError("refused on purpose")
    """)
    assert "ValueError: refused on purpose" in run.error
    measured = runs.Measured(pltrs_side)
    measured.add(run)
    assert measured.error == run.error and measured.runs == []


def test_the_description_is_the_docstrings_first_line(tmp_path):
    script = tmp_path / "plot.py"
    script.write_text('"""  Three clusters\n\nMore.\n"""\nx = 1\n', encoding="utf-8")
    assert compare.describe(script) == "Three clusters"
    script.write_text("x = 1\n", encoding="utf-8")
    assert compare.describe(script) == "plot.py"
    script.write_text('""""""\n', encoding="utf-8")
    assert compare.describe(script) == "plot.py"


def test_every_example_has_a_description():
    examples = sorted((TOOL / "examples").glob("*.py"))
    assert len(examples) == 10
    for example in examples:
        assert compare.describe(example) != example.name


@pytest.fixture(scope="module")
def clip():
    """A clip of a red matplotlib picture at 1.5 s and a blue pltrs one
    at 0.5 s, in Pillow's own font."""
    sides = (
        frames.Side("matplotlib", "3.11.2", 1.5, Image.new("RGB", (300, 200), MATPLOTLIB_RED)),
        frames.Side("pltrs", "0.1.0", 0.5, Image.new("RGB", (300, 200), PLTRS_BLUE)),
    )
    return frames.make_clip(frames.background(), sides, "A test", frames.load_fonts())


def card_colours(clip, t):
    """The colours at the centres of the left and right cards at *t*."""
    frame = frames.frame_at(clip, t)
    middle = frames.TOP + clip.cards[0].height // 2 + frames.STRIP // 2
    return [frame.getpixel((centre, middle)) for centre in frames.card_centres(clip)]


def test_no_card_is_in_the_frame_before_either_time(clip):
    left, right = card_colours(clip, 0.2)
    assert left not in (MATPLOTLIB_RED, PLTRS_BLUE)
    assert right not in (MATPLOTLIB_RED, PLTRS_BLUE)
    assert frames.caption_at(clip, 0.2) == "A test"


def test_the_faster_card_alone_is_in_the_frame_between_the_times(clip):
    left, right = card_colours(clip, 1.0)
    assert left != MATPLOTLIB_RED
    assert right == PLTRS_BLUE


def test_both_cards_and_the_verdict_are_in_the_frame_at_the_end(clip):
    end = clip.duration - 1 / frames.FPS
    assert card_colours(clip, end) == [MATPLOTLIB_RED, PLTRS_BLUE]
    assert frames.caption_at(clip, end) == "pltrs had the figure 1.00 s earlier"


def test_the_clip_lasts_past_the_later_card(clip):
    assert clip.duration == pytest.approx(1.5 + frames.POP + frames.HOLD)
    assert sum(1 for _ in frames.frames(clip, fps=2)) == round(clip.duration * 2)


def test_pictures_are_the_same_when_their_pixels_are(tmp_path):
    first, second, other = (tmp_path / f"{name}.png" for name in ("first", "second", "other"))
    Image.new("RGB", (4, 4), (10, 20, 30)).save(first)
    Image.new("RGB", (4, 4), (10, 20, 30)).save(second)
    Image.new("RGB", (4, 4), (10, 20, 40)).save(other)
    assert pictures.compare([first], [second]) == {"rms": 0.0, "verdict": "the same picture"}
    assert pictures.compare([first], [other])["rms"] == pytest.approx(10 / 3 ** 0.5)
    assert pictures.compare([first], [])["verdict"] == "no picture to compare"


def test_without_ffmpeg_pillow_writes_a_smaller_animation(clip, tmp_path):
    encoder = encode.pillow_encoder()
    assert encoder.ffmpeg is None and encoder.suffix in (".webp", ".gif")
    path = tmp_path / f"clip{encoder.suffix}"
    encoder.write(frames.frames(clip, fps=2), path)
    with Image.open(path) as written:
        assert written.size == (frames.W // 2, frames.H // 2)
        assert written.n_frames == round(clip.duration * 2)
