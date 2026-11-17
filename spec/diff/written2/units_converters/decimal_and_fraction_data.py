# Plot decimal.Decimal and fractions.Fraction data with the DecimalConverter and without a converter; tick labels and limits.
import decimal
import fractions
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

dec = [decimal.Decimal("0.1") * k for k in range(6)]
frac = [fractions.Fraction(k, 3) for k in range(6)]
fig, axs = plt.subplots(1, 3, figsize=(8, 3))
l1, = axs[0].plot(range(6), dec, "o-")
l2, = axs[1].plot(range(6), frac, "s-")
axs[2].bar(range(3), [decimal.Decimal("1.25"), decimal.Decimal("2.5"), decimal.Decimal("0.75")])
fig.canvas.draw()
print(type(axs[0].yaxis.converter).__name__, type(axs[1].yaxis.converter).__name__ if axs[1].yaxis.converter else None)
print([round(float(v), 4) for v in axs[0].get_ylim()], [round(float(v), 4) for v in axs[1].get_ylim()])
print([t.get_text() for t in axs[0].get_yticklabels()][:4])
print(type(l1.get_ydata()[0]).__name__, type(l1.get_ydata(orig=False)[0]).__name__)
print(type(l2.get_ydata()[0]).__name__, type(l2.get_ydata(orig=False)[0]).__name__)
print([round(float(p.get_height()), 3) for p in axs[2].patches])
axs[0].axhline(decimal.Decimal("0.25"), color="r")
axs[0].set_ylim(decimal.Decimal("0"), decimal.Decimal("0.6"))
fig.canvas.draw()
print([round(float(v), 4) for v in axs[0].get_ylim()])
sc = axs[1].scatter(frac, frac)
print(np.asarray(sc.get_offsets()).dtype, np.round(np.asarray(sc.get_offsets(), float)[:2], 4).tolist())
fig.savefig("decimals.png")
txt = axs[0].annotate("mid", (decimal.Decimal(2), decimal.Decimal("0.2")))
fig.canvas.draw()
print(type(txt.xy[0]).__name__, [round(v, 2) for v in txt.get_window_extent().bounds][:2])
