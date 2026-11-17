# Context managers a helper library offers: a figure context that saves and closes, nested with rc_context and style.context.
import contextlib
import io
import os
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


@contextlib.contextmanager
def figure(path, *, style="default", rc=None, **subplot_kw):
    with plt.style.context(style), plt.rc_context(rc or {}):
        fig, ax = plt.subplots(**subplot_kw)
        try:
            yield ax
        finally:
            fig.savefig(path)
            plt.close(fig)


os.makedirs("out", exist_ok=True)
with figure("out/a.png", figsize=(3, 2)) as ax:
    ax.plot([1, 2, 3])
    print(plt.rcParams["axes.facecolor"], len(plt.get_fignums()))
with figure("out/b.png", style="ggplot", rc={"lines.linewidth": 4}) as ax:
    ln, = ax.plot([3, 2, 1])
    print(plt.rcParams["axes.facecolor"], ln.get_linewidth(), ln.get_color())
try:
    with figure("out/c.png") as ax:
        ax.plot([1])
        raise RuntimeError("plotting failed")
except RuntimeError as e:
    print("RuntimeError", e, os.path.exists("out/c.png"), plt.get_fignums())
with figure("out/d.png", style=["dark_background", {"axes.grid": True}]) as ax:
    ax.plot([0, 1])
    print(plt.rcParams["axes.grid"], plt.rcParams["figure.facecolor"])
print(sorted(os.listdir("out")), plt.rcParams["axes.facecolor"], plt.rcParams["lines.linewidth"])
with contextlib.ExitStack() as stack:
    axes = [stack.enter_context(figure(f"out/e{k}.png", figsize=(2, 2))) for k in range(3)]
    for k, ax in enumerate(axes):
        ax.bar([0], [k + 1])
    print(len(plt.get_fignums()))
print(len(plt.get_fignums()), len(os.listdir("out")))
