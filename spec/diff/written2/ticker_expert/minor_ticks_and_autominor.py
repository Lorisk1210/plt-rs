# Minor ticks: AutoMinorLocator with n, minorticks_on per scale, rcParams xtick.minor.ndivs, and minor tick labels with formatters.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import ticker

fig, axs = plt.subplots(2, 3, figsize=(8, 4), dpi=50)
scales = [("linear", (0, 10)), ("log", (1, 1e4)), ("symlog", (-100, 100)), ("logit", (0.01, 0.99)), ("asinh", (-50, 50)),
          ("linear", (0, 1))]
for ax, (scale, lim) in zip(axs.flat, scales):
    ax.set_xscale(scale)
    ax.set_xlim(*lim)
    ax.minorticks_on()
fig.canvas.draw()
for ax, (scale, lim) in zip(axs.flat, scales):
    minors = ax.xaxis.get_minorticklocs()
    print(scale, type(ax.xaxis.get_minor_locator()).__name__, len(minors), [float(f"{v:.4g}") for v in minors[:4]])
ax = axs[1, 2]
ax.xaxis.set_minor_locator(ticker.AutoMinorLocator(n=4))
ax.xaxis.set_minor_formatter(ticker.FormatStrFormatter("%.3f"))
ax.tick_params(axis="x", which="minor", labelsize=5)
fig.canvas.draw()
print(len(ax.xaxis.get_minorticklocs()), [t.get_text() for t in ax.xaxis.get_minorticklabels()][:3])
with plt.rc_context({"xtick.minor.ndivs": 3, "xtick.minor.visible": True}):
    f2, a2 = plt.subplots()
    a2.set_xlim(0, 10)
    f2.canvas.draw()
    print(len(a2.xaxis.get_minorticklocs()), type(a2.xaxis.get_minor_locator()).__name__)
ax.minorticks_off()
print(len(ax.xaxis.get_minorticklocs()), type(ax.xaxis.get_minor_locator()).__name__)
fig.savefig("minors.png")
