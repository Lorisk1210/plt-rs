# Every fillable marker with every fillstyle and markerfacecoloralt, plus markevery variants; reads back Line2D's marker state.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.markers import MarkerStyle

fills = MarkerStyle.fillstyles
filled = Line2D.filled_markers
print(fills, filled)
fig, ax = plt.subplots(figsize=(6, 5))
lines = []
for r, fs in enumerate(fills):
    for c, m in enumerate(filled):
        ln, = ax.plot(c, r, marker=m, fillstyle=fs, ms=12, mfc="C0", mfcalt="C3", mec="k", ls="")
        lines.append(ln)
ax.set_xlim(-1, len(filled))
ax.set_ylim(-1, len(fills))
fig.savefig("fillstyles.png")
print(lines[-1].get_fillstyle(), lines[-1].get_markerfacecoloralt(), lines[len(filled)].get_fillstyle())
x = np.linspace(0, 10, 101)
fig2, ax2 = plt.subplots(figsize=(5, 3))
specs = [5, (3, 7), [0, 10, 50], slice(2, 60, 9), 0.1, (0.05, 0.2), np.arange(101) % 13 == 0]
for k, me in enumerate(specs):
    ln, = ax2.plot(x, np.sin(x) + k, "o-", markevery=me, ms=4)
    fig2.canvas.draw()
    path, affine = ln._get_transformed_path().get_transformed_points_and_affine()
    print(type(me).__name__, ln.get_markevery() is me)
fig2.savefig("markevery.png")
ln = ax2.lines[0]
ln.set_marker(MarkerStyle("s", fillstyle="left"))
print(ln.get_marker(), ln.get_fillstyle(), ln.get_markersize())
ln.set_fillstyle("none")
print(ln.get_fillstyle(), ln.get_markerfacecolor())
