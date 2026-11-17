# clabel with fmt as string, dict, Formatter and callable, inline on/off, manual positions; label texts and positions.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import ticker

x = np.linspace(-3, 3, 80)
X, Y = np.meshgrid(x, x)
Z = np.hypot(X, Y)
fig, axs = plt.subplots(2, 2, figsize=(6, 6), dpi=60)
fmts = ["%1.1f", {1.0: "one", 2.0: "two", 3.0: "three"}, ticker.StrMethodFormatter("{x:.2f} m"), lambda v: f"<{v:g}>"]
for ax, fmt in zip(axs.flat, fmts):
    cs = ax.contour(X, Y, Z, levels=[1, 2, 3])
    labels = cs.clabel(fmt=fmt, inline=fmt is not fmts[1], fontsize=8)
    print(type(fmt).__name__, [t.get_text() for t in labels], len(cs.labelTexts), cs.labelLevelList)
cs = axs[0, 0].contour(X, Y, Z, levels=[0.5, 1.5], colors="r")
manual = cs.clabel(manual=[(0.5, 0), (0, 1.5)], fmt="%.2f")
fig.canvas.draw()
print([t.get_text() for t in manual], [[round(v, 2) for v in t.get_position()] for t in manual])
print([round(t.get_rotation(), 1) for t in manual])
cs2 = axs[1, 1].contour(X, Y, Z, levels=[2.5])
lab = cs2.clabel(inline_spacing=20, use_clabeltext=True, colors="g", rightside_up=False)
fig.canvas.draw()
print(type(lab[0]).__name__, lab[0].get_color(), len(cs2.get_paths()[0].vertices))
fig.savefig("clabels.png")
for t in manual:
    t.remove()
print(len(cs.labelTexts), len(axs[0, 0].texts))
cs.set_linewidth(2)
print(np.asarray(cs.get_linewidth()).tolist())
cs.set_color("b")
print(np.round(cs.get_edgecolor()[0], 3).tolist())
