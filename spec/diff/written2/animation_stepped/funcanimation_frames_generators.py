# FuncAnimation with frames as an int, a list, a generator function and an iterator with save_count; values each frame receives.
import itertools
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import animation
import warnings

fig, ax = plt.subplots(figsize=(2, 2), dpi=30)
ln, = ax.plot([0, 1], [0, 1])


def run(frames, **kw):
    seen = []

    def update(f):
        seen.append(f)
        ln.set_ydata([0, len(seen)])
        return (ln,)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        anim = animation.FuncAnimation(fig, update, frames=frames, repeat=False, **kw)
        anim.save("run.gif", writer="pillow", fps=10)
    return seen, [re.sub(r"0x[0-9a-f]+", "0x?", str(w.message))[:70] for w in caught], anim


def gen():
    for k in range(3):
        yield k * 10


for label, frames, kw in [
    ("int", 4, {}),
    ("list", ["a", "b", "c"], {}),
    ("genfunc", gen, {}),
    ("iterator+save_count", itertools.count(100), dict(save_count=5)),
    ("range", range(2, 8, 2), dict(cache_frame_data=False)),
]:
    seen, warns, anim = run(frames, **kw)
    print(label, seen, warns, anim._save_count)
seen, warns, anim = run(lambda: iter([1, 2]))
print(seen)
print(len(anim._save_seq) if hasattr(anim, "_save_seq") else None)
