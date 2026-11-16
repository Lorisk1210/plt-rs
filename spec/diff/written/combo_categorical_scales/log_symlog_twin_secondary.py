# Log x with a secondary top axis in decades, symlog y on a twin, and a logit inset: scales combined in one figure.
import numpy as np
import matplotlib.pyplot as plt

x = np.logspace(-2, 3, 200)
fig, ax = plt.subplots(figsize=(8, 5))
ax.semilogx(x, np.log10(x) ** 2, label="log² x")
sec = ax.secondary_xaxis("top", functions=(np.log10, lambda d: 10 ** d))
sec.set_xlabel("decade")
tw = ax.twinx()
tw.plot(x, np.sinh(np.log10(x) * 2) * 10, color="C1")
tw.set_yscale("symlog", linthresh=1)
ins = ax.inset_axes([0.08, 0.55, 0.3, 0.35])
p = np.linspace(0.01, 0.99, 50)
ins.plot(p, p ** 2)
ins.set_xscale("logit")
fig.canvas.draw()
print(ax.get_xscale(), tw.get_yscale(), ins.get_xscale(), [t.get_text() for t in sec.xaxis.get_ticklabels()][:4])
fig.savefig("scales.png", dpi=50)
