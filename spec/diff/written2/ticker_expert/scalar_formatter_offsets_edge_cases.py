# ScalarFormatter offset and order-of-magnitude decisions over many ranges, with useOffset thresholds and rcParams limits.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

ranges = [(0, 1), (0, 1e5), (1e5, 1e5 + 1), (-1e-6, 1e-6), (123456, 123460), (0.999, 1.001), (1e9, 2e9), (-5e4, 5e4),
          (99999.5, 100000.5), (1e-12, 5e-12)]
fig, ax = plt.subplots(figsize=(5, 2), dpi=60)
for lo, hi in ranges:
    ax.set_xlim(lo, hi)
    fig.canvas.draw()
    fmt = ax.xaxis.get_major_formatter()
    print((lo, hi), [t.get_text() for t in ax.get_xticklabels()][:3], repr(ax.xaxis.get_offset_text().get_text()),
          fmt.offset, fmt.orderOfMagnitude)
with plt.rc_context({"axes.formatter.limits": (-2, 2), "axes.formatter.offset_threshold": 2,
                     "axes.formatter.useoffset": True, "axes.formatter.use_mathtext": True}):
    f2, a2 = plt.subplots(figsize=(5, 2), dpi=60)
    for lo, hi in ranges[:5]:
        a2.set_xlim(lo, hi)
        f2.canvas.draw()
        print("rc", (lo, hi), [t.get_text() for t in a2.get_xticklabels()][:2], repr(a2.xaxis.get_offset_text().get_text()))
ax.ticklabel_format(style="plain", useOffset=False)
ax.set_xlim(1e9, 2e9)
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()][:2])
ax.ticklabel_format(axis="x", style="sci", scilimits=(0, 0))
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()][:2], repr(ax.xaxis.get_offset_text().get_text()))
ax.ticklabel_format(useMathText=True)
fig.canvas.draw()
print(repr(ax.xaxis.get_offset_text().get_text()))
