# Bar chart race: horizontal bars re-sorted each frame with value labels and a year label, saved as GIF.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation

rng = np.random.default_rng(102)
names = ["Ada", "Bo", "Cy", "Di", "Ed"]
values = np.cumsum(rng.integers(1, 10, (8, 5)), axis=0)
colors = dict(zip(names, plt.cm.tab10.colors))
fig, ax = plt.subplots(figsize=(5, 3), dpi=50)
def draw(frame):
    ax.clear()
    order = np.argsort(values[frame])
    bars = ax.barh([names[i] for i in order], values[frame][order], color=[colors[names[i]] for i in order])
    ax.bar_label(bars, padding=2)
    ax.set_xlim(0, values.max() * 1.15)
    ax.text(0.95, 0.05, str(2016 + frame), transform=ax.transAxes, ha="right", fontsize=14)
    return bars
anim = FuncAnimation(fig, draw, frames=8)
anim.save("race.gif", writer="pillow", fps=3)
print(values[-1].tolist(), [t.get_text() for t in ax.texts][-1])
