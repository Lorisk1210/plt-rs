# secondary_xaxis/secondary_yaxis with forward/inverse function pairs, transforms, and locations; tick values on each.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4), dpi=60, layout="constrained")
x = np.linspace(1, 100, 50)
ax.plot(x, np.log10(x))
ax.set_xlabel("frequency [Hz]")
ax.set_xlim(1, 100)
sec = ax.secondary_xaxis("top", functions=(lambda f: 1 / f, lambda p: 1 / p))
sec.set_xlabel("period [s]")
sec2 = ax.secondary_yaxis("right", functions=(lambda v: v * 20, lambda d: d / 20))
sec3 = ax.secondary_xaxis(-0.25, functions=(np.sqrt, np.square))
sec4 = ax.secondary_yaxis(1.2, transform=ax.transAxes)
fig.canvas.draw()
for s in (sec, sec2, sec3, sec4):
    axis = s.xaxis if s in (sec, sec3) else s.yaxis
    print(type(s).__name__, [t.get_text() for t in axis.get_ticklabels()][:4], [round(v, 4) for v in (s.get_xlim() if axis is s.xaxis else s.get_ylim())])
ax.set_xlim(10, 50)
fig.canvas.draw()
print([round(v, 4) for v in sec.get_xlim()], [round(v, 4) for v in sec3.get_xlim()])
sec.set_location("bottom")
sec.set_functions((lambda f: f * 2, lambda f: f / 2))
fig.canvas.draw()
print([round(v, 4) for v in sec.get_xlim()], [t.get_text() for t in sec.get_xticklabels()][:3])
try:
    ax.secondary_xaxis("left")
except ValueError as e:
    print("ValueError", str(e)[:60])
print(len(fig.axes), len(ax.child_axes))
fig.savefig("secondary.png")
