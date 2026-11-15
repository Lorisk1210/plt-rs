"""Programs recursing through pltrs (Rust)'s drawing end in matplotlib's
RecursionError, on any stack, and leave the interpreter usable.

Each round of such a recursion runs native frames between CPython's own
checks, so the native stack could end first and the process die (SIGSEGV
on Linux; under Rosetta on macOS a hang no signal ends). pltrs refuses a
nested native call while little of the thread's stack is left
(``binding/stack.rs``). Each program runs in a
process of its own, so that a crash fails its test and no other; each is
run on the main thread, on threads of 512 KiB and 16 MiB
(`threading.stack_size`) and, where the system has them, under
``ulimit -s 1024`` and ``ulimit -s unlimited``. matplotlib 3.11 raises
``RecursionError: maximum recursion depth exceeded`` for every one of
them, everywhere, and draws the figure again once the recursing part is
gone. pltrs' message starts with that line; on a thread whose stack is
smaller than 1 MiB it goes on to say why and what to do.
"""

import ast
import os
import re
import subprocess
import sys
import textwrap

import pytest

import pltrs

try:
    import resource
except ImportError:  # Windows
    resource = None

#: How long one program may run, in seconds.
TIMEOUT = 300

#: What the programs share: the figure, then after the recursion the
#: figure drawn again without the recursing part, and a new figure saved.
PROGRAM = """
import io, threading
import pltrs
pltrs.use("agg")
import pltrs.pyplot as plt
from pltrs import artist, ticker
from pltrs.axes import Axes
from pltrs.figure import Figure

def run():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    trigger, undo = recursing(fig, ax)
    try:
        trigger()
    except RecursionError as error:
        outcome = "RecursionError: " + str(error).splitlines()[0]
    else:
        outcome = "no error"
    undo()
    fig.savefig(io.BytesIO(), format="png")
    other, other_ax = plt.subplots()
    other_ax.plot([1, 2])
    other.savefig(io.BytesIO(), format="pdf")
    print(outcome, "| usable", flush=True)

{recursing}

stack = {stack}
if stack is None:
    run()
else:
    threading.stack_size(stack)
    thread = threading.Thread(target=run)
    thread.start()
    thread.join()
"""

#: The recursing parts: each defines ``recursing(fig, ax)``, which sets it
#: up and returns what sets it off and what removes it.
RECURSING = {
    # The fuzzer's seed 22: each save draws, each draw formats the ticks,
    # each tick saves.
    "a formatter saving its figure": """
        def recursing(fig, ax):
            def label(value, pos):
                fig.savefig(io.BytesIO(), format="svg")
                return str(value)
            ax.xaxis.set_major_formatter(ticker.FuncFormatter(label))
            return (lambda: fig.savefig(io.BytesIO(), format="png"),
                    lambda: ax.xaxis.set_major_formatter(ticker.ScalarFormatter()))
    """,
    # The same, with tick labels that are formulas nesting fractions as
    # deep as matplotlib's parser takes them (20 levels), each new, so
    # that each round typesets one (fuzzer seed 600278 overflowed the
    # stack typesetting a formula at the bottom of a nested draw).
    "a formatter saving its figure, labelled by deep formulas": """
        def recursing(fig, ax):
            made = []
            def label(value, pos):
                made.append(value)
                fig.savefig(io.BytesIO(), format="svg")
                return "$" + r"\\frac{" * 20 + str(len(made)) + "}{x}" * 20 + "$"
            ax.xaxis.set_major_formatter(ticker.FuncFormatter(label))
            return (lambda: fig.savefig(io.BytesIO(), format="png"),
                    lambda: ax.xaxis.set_major_formatter(ticker.ScalarFormatter()))
    """,
    # Recorded and replayed natively, the artist's draw nests no Python
    # frames: CPython's limit does not end it.
    "an artist drawing its Axes": """
        def recursing(fig, ax):
            drawer = type("A", (artist.Artist,),
                          {"draw": lambda self, renderer: self.axes.draw(renderer)})()
            ax.add_artist(drawer)
            return lambda: fig.savefig(io.BytesIO(), format="png"), drawer.remove
    """,
    "an artist drawing its figure": """
        def recursing(fig, ax):
            drawer = type("A", (artist.Artist,),
                          {"draw": lambda self, renderer: self.figure.draw(renderer)})()
            ax.add_artist(drawer)
            return lambda: fig.savefig(io.BytesIO(), format="png"), drawer.remove
    """,
    "a locator drawing its figure": """
        def recursing(fig, ax):
            class Drawing(ticker.MaxNLocator):
                def __call__(self):
                    fig.canvas.draw()
                    return super().__call__()
            ax.yaxis.set_major_locator(Drawing())
            return (fig.canvas.draw,
                    lambda: ax.yaxis.set_major_locator(ticker.AutoLocator()))
    """,
    # canvas.draw, unlike draw_idle, has no guard against being called
    # from its own draw event.
    "a draw event drawing again": """
        def recursing(fig, ax):
            cid = fig.canvas.mpl_connect("draw_event", lambda event: fig.canvas.draw())
            return fig.canvas.draw, lambda: fig.canvas.mpl_disconnect(cid)
    """,
    # Members of the program's classes that pltrs (Rust) calls from inside
    # its draws (`_program_classes.takes_from_program`).
    "a locator's tick_values drawing its figure": """
        def recursing(fig, ax):
            class Drawing(ticker.MaxNLocator):
                def tick_values(self, vmin, vmax):
                    fig.canvas.draw()
                    return super().tick_values(vmin, vmax)
            ax.xaxis.set_major_locator(Drawing())
            return (fig.canvas.draw,
                    lambda: ax.xaxis.set_major_locator(ticker.AutoLocator()))
    """,
    "an Axes.draw drawing its canvas": """
        def recursing(fig, ax):
            class Drawing(Axes):
                drawing = False
                def draw(self, renderer):
                    if self.drawing:
                        self.figure.canvas.draw()
                    return super().draw(renderer)
            other = fig.add_subplot(2, 1, 2, axes_class=Drawing)
            other.plot([1, 2])
            def stop():
                other.drawing = False
                other.remove()
            def trigger():
                other.drawing = True
                fig.canvas.draw()
            return trigger, stop
    """,
    # A class measuring its artists itself is measured from inside a tight
    # save's layout.
    "an artist measuring its figure": """
        def recursing(fig, ax):
            class Measuring(artist.Artist):
                def draw(self, renderer):
                    pass
                def get_window_extent(self, renderer=None):
                    return fig.get_tightbbox(renderer)
            measuring = Measuring()
            ax.add_artist(measuring)
            # Clipped to the Axes, matplotlib would not measure it.
            measuring.set_clip_on(False)
            return (lambda: fig.savefig(io.BytesIO(), format="png", bbox_inches="tight"),
                    measuring.remove)
    """,
    # canvas draws and prints call the program's Figure.draw.
    "a Figure.draw drawing its canvas": """
        def recursing(fig, ax):
            class Drawing(Figure):
                def draw(self, renderer):
                    if self.drawing:
                        self.canvas.draw()
                    return super().draw(renderer)
            drawing = plt.figure(FigureClass=Drawing)
            drawing.drawing = False
            drawing.subplots().plot([1, 2])
            def stop():
                drawing.drawing = False
                drawing.savefig(io.BytesIO(), format="png")
            def trigger():
                drawing.drawing = True
                drawing.savefig(io.BytesIO(), format="png")
            return trigger, stop
    """,
}

#: The stacks the programs run on: a thread's stack size (None for the
#: main thread) and the soft stack limit the process starts with (None
#: for the inherited one).
STACKS = {
    "main thread": (None, None),
    "thread of 512 KiB": (512 << 10, None),
    "thread of 16 MiB": (16 << 20, None),
    "ulimit -s 1024": (None, 1 << 20),
    "ulimit -s unlimited": (None, "unlimited"),
}


#: The address space a program may take where the system caps it (Linux):
#: the fuzzer's cap, so that a recursion growing an unlimited stack fails
#: its test instead of exhausting the machine's memory.
MEMORY = 6 << 30


def _limits(stack):
    """What sets the child's limits before it starts the interpreter: its
    soft stack limit *stack* (None for the inherited one) and, on Linux,
    its address space."""
    if resource is None:
        if stack is not None:
            pytest.skip("stack limits are POSIX's")
        return None
    _, hard = resource.getrlimit(resource.RLIMIT_STACK)
    soft = resource.RLIM_INFINITY if stack == "unlimited" else stack
    if stack is not None and hard != resource.RLIM_INFINITY and (
            soft == resource.RLIM_INFINITY or soft > hard):
        pytest.skip(f"the hard stack limit ({hard}) is below {stack}")

    def limit():
        if stack is not None:
            resource.setrlimit(resource.RLIMIT_STACK, (soft, hard))
        if sys.platform.startswith("linux"):
            resource.setrlimit(resource.RLIMIT_AS, (MEMORY, MEMORY))
    return limit


def _run(source, cwd, stack=None):
    """The last line *source* printed, run in a new interpreter with the
    soft stack limit *stack*; it must end normally."""
    proc = subprocess.run(
        [sys.executable, "-c", source], cwd=cwd, capture_output=True, text=True,
        timeout=TIMEOUT, preexec_fn=_limits(stack), env={**os.environ, "MPLBACKEND": "agg"})
    assert proc.returncode == 0, proc.stderr[-3000:]
    return proc.stdout.splitlines()[-1]


@pytest.mark.parametrize("stack", STACKS)
@pytest.mark.parametrize("program", RECURSING)
def test_a_recursion_through_drawing_raises_recursion_error(program, stack, tmp_path):
    size, limit = STACKS[stack]
    source = PROGRAM.format(recursing=textwrap.dedent(RECURSING[program]), stack=size)
    assert _run(source, tmp_path, limit) == (
        "RecursionError: maximum recursion depth exceeded | usable")


@pytest.mark.parametrize("kib", [128, 512])
@pytest.mark.parametrize("save", ["png", "pdf"])
def test_nested_calls_on_a_small_stack_are_not_refused(save, kib, tmp_path):
    # The reserve leaves a thread of 128 KiB (musl's default) room for a
    # formatter's own native calls during a tight save: only recursion is
    # refused.
    source = textwrap.dedent(f"""
        import io, threading
        import pltrs
        pltrs.use("agg")
        import pltrs.pyplot as plt
        from pltrs import ticker

        def run():
            fig, ax = plt.subplots()
            ax.plot([1, 2, 3])
            ax.xaxis.set_major_formatter(ticker.FuncFormatter(
                lambda value, pos: f"{{value:g}}/{{ax.get_xlim()[1]:g}}"))
            fig.savefig(io.BytesIO(), format={save!r}, bbox_inches="tight")
            print([label.get_text() for label in ax.get_xticklabels()][:2])

        threading.stack_size({kib} << 10)
        thread = threading.Thread(target=run)
        thread.start()
        thread.join()
    """)
    assert _run(source, tmp_path) == "['-0.25/2.1', '0/2.1']"


@pytest.mark.parametrize("levels", [10, 20])
def test_a_deep_formula_on_a_small_stack_is_drawn(levels, tmp_path):
    # Typesetting a formula goes deeper the more it nests (pltrs: about 7
    # KiB a level of fractions); matplotlib's parser takes 20 levels and
    # refuses 30, and draws 20 on a thread of 128 KiB (musl's). pltrs
    # typesets a formula on a stack of its own where the thread's has too
    # little left for its nesting (`pltrs_text::deep`).
    source = textwrap.dedent(f"""
        import io, threading
        import pltrs
        pltrs.use("agg")
        import pltrs.pyplot as plt

        def run():
            fig, ax = plt.subplots()
            ax.set_title("$" + r"\\frac{{" * {levels} + "x" + "}}{{y}}" * {levels} + "$")
            fig.savefig(io.BytesIO(), format="png")
            print("drawn")

        threading.stack_size(128 << 10)
        thread = threading.Thread(target=run)
        thread.start()
        thread.join()
    """)
    assert _run(source, tmp_path) == "drawn"


@pytest.mark.skipif(pltrs.__name__ != "pltrs", reason="pltrs' own message")
def test_a_draw_inside_a_draw_on_a_small_stack_says_what_to_do(tmp_path):
    # A formatter saving its figure once, on a thread of 128 KiB: pltrs'
    # two draws need more stack than that (matplotlib's fit in it), and the
    # error names the stack and the way to give threads more.
    source = textwrap.dedent("""
        import io, threading
        import pltrs
        pltrs.use("agg")
        import pltrs.pyplot as plt
        from pltrs import ticker

        def run():
            fig, ax = plt.subplots()
            saved = []
            def label(value, pos):
                if not saved:
                    saved.append(fig.savefig(io.BytesIO(), format="png"))
                return f"{value:g}"
            ax.xaxis.set_major_formatter(ticker.FuncFormatter(label))
            try:
                fig.savefig(io.BytesIO(), format="png")
            except RecursionError as error:
                print(repr(str(error)))

        threading.stack_size(128 << 10)
        thread = threading.Thread(target=run)
        thread.start()
        thread.join()
    """)
    first, rest = ast.literal_eval(_run(source, tmp_path)).split("\n", 1)
    assert first == "maximum recursion depth exceeded"
    # 128 KiB as asked, and what the C library adds to it: musl 131,
    # glibc up to 136; macOS reports 12 KiB more than asked for every size
    # (its 16 KiB guard page apart, `pthread_get_stacksize_np`).
    low, high = (140, 140) if sys.platform == "darwin" else (128, 136)
    size = re.search(r"pltrs: this thread's stack is (\d+) KiB", rest)
    assert size is not None and low <= int(size.group(1)) <= high, rest
    assert "threading.stack_size(1048576)" in rest


@pytest.mark.skipif(not sys.platform.startswith("linux") or pltrs.__name__ != "pltrs",
                    reason="pltrs' report of a fault, on Linux")
def test_the_fuzzer_keeps_a_crashs_fault_address_and_first_lines(tmp_path):
    # A fuzz worker sets PLTRS_REPORT_FAULTS: pltrs writes the address a
    # fatal fault touched before faulthandler says where the program was,
    # and the runner places it against the worker's stack.
    from fuzz import runner

    worker = runner.Worker(tmp_path)
    _, bugs = worker.run({"id": 0, "lines": ["import ctypes", "ctypes.string_at(16)"]})
    assert [bug.signature for bug in bugs] == ["crash:-11"]
    first = bugs[0].detail.splitlines()[:4]
    assert first[0] == "fault at 0x10, outside the main thread's stack", first
    assert "pltrs: fatal signal 11 at address 0x10" in first, first
    assert "Fatal Python error: Segmentation fault" in first, first
    lowest, highest = worker.stack
    # The span ulimit -s gives (under qemu-user, qemu's own stack instead).
    assert lowest < highest
    below = runner._fault_note("pltrs: fatal signal 11 at address 0x1000\n", [0x3000, 0x5000])
    assert below == "fault at 0x1000, 8.0 KiB below the main thread's stack limit: a stack overflow\n"
    kept = runner._kept([str(n) for n in range(100)])
    assert kept[:2] == ["0", "1"] and kept[-1] == "99" and "[45 lines left out]" in kept


#: How much of its reserve a nested draw must leave: what it takes of the
#: stack below the check that let its native call in, when what goes
#: deep with its input (shaping a new line of text, typesetting a
#: formula) runs on a stack of its own (`PLTRS_DEEP_WORK_APART`), must be
#: at least this much less than the reserve the guard checks for
#: (`binding/stack.rs`), on the system the test runs on: what the call
#: does below that check, after the calls it makes are refused too, must
#: fit, or the process dies; the margin is for the draws these four
#: scenes do not take. The calls into Python a draw makes are refused
#: before CPython 3.14's own check would (48 KiB left against its 32), so
#: the message does not depend on the reserve. Measured (KiB below the
#: formatter's native call, development / optimised build): Linux x86-64,
#: reserve 64: PNG 39.3 / 37.4, SVG 47.1 / 45.6, PDF 42.3 / 42.9, EPS
#: 42.2 / 40.1 (61 and more before the renderers, the figure state a
#: layout works on and the axis draw's locals left the frames the layout
#: runs below). A change taking more of the stack on that path fails here,
#: in its author's smoke tests, not in a draw on a small thread or a merge
#: check.
STRETCH_MARGIN = 12 << 10

#: The scenes measured: each figure saved with a formatter that saves it
#: again once (a nested native call), in each format.
STRETCH_SCENES = {
    "png, tight": dict(format="png", bbox_inches="tight"),
    "svg, constrained": dict(format="svg"),
    "pdf, constrained": dict(format="pdf"),
    "eps, constrained": dict(format="eps"),
}

#: The program measuring the stretch: the stack of a thread of 1 MiB is
#: painted below the formatter's frame before the nested save, and
#: scanned for the deepest byte the save wrote; pltrs tells where the
#: stack is and the reserve (`_stack_room`). Prints the stretch and the
#: reserve.
STRETCH_PROGRAM = """
import ctypes, io, threading
import numpy as np
import pltrs
pltrs.use("agg")
import pltrs.pyplot as plt
from pltrs import _native, ticker

PATTERN = 0xA5

def run():
    fig = plt.figure(layout="constrained")
    ax = fig.subplots()
    ax.plot(np.linspace(0, 1, 20) ** 2, label="line")
    ax.set_title(r"$\\sum_i x_i$")
    ax.set_xlabel("x")
    ax.legend()
    fig.colorbar(ax.imshow(np.arange(16).reshape(4, 4)), ax=ax)
    kwargs = {kwargs!r}
    state = {{"armed": False, "stretch": 0, "reserve": 0}}
    def label(value, pos):
        if state["armed"]:
            state["armed"] = False
            here, end, state["reserve"] = _native._stack_room()
            start, top = end + 4096, here - (16 << 10)
            ctypes.memset(start, PATTERN, top - start)
            fig.savefig(io.BytesIO(), **kwargs)
            data = ctypes.string_at(start, top - start)
            deepest = start + len(data) - len(data.lstrip(bytes([PATTERN])))
            state["stretch"] = max(state["stretch"], here - deepest)
        return f"{{value:.2f}}"
    ax.xaxis.set_major_formatter(ticker.FuncFormatter(label))
    for _ in range(2):
        state["armed"] = True
        fig.savefig(io.BytesIO(), **kwargs)
    print(state["stretch"], state["reserve"])

threading.stack_size(1 << 20)
thread = threading.Thread(target=run)
thread.start()
thread.join()
"""


@pytest.mark.skipif(sys.platform == "win32" or pltrs.__name__ != "pltrs",
                    reason="pltrs' own stack, painted below a thread's stack pointer "
                           "(Windows commits a thread's stack as it grows)")
@pytest.mark.parametrize("scene", STRETCH_SCENES)
def test_a_nested_draw_leaves_a_margin_of_its_reserve(scene, tmp_path, monkeypatch):
    monkeypatch.setenv("PLTRS_DEEP_WORK_APART", "1")
    source = STRETCH_PROGRAM.format(kwargs=STRETCH_SCENES[scene])
    stretch, reserve = map(int, _run(source, tmp_path).split())
    assert 0 < stretch <= reserve - STRETCH_MARGIN, (
        f"{stretch / 1024:.1f} KiB of a reserve of {reserve / 1024:.0f} KiB")
