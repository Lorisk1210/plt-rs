# Read errorbar caps and bar segments back (container.lines) to draw a shaded band through the same bounds.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(6)
y = np.array([2.0, 2.5, 3.1, 2.8, 3.6, 4.0])
fig, ax = plt.subplots(figsize=(6, 4))
eb = ax.errorbar(x, y, yerr=0.4 + 0.05 * x, fmt="o", capsize=4)
segs = eb.lines[2][0].get_segments()
lo = np.array([s[0][1] for s in segs]); hi = np.array([s[1][1] for s in segs])
ax.fill_between(x, lo, hi, alpha=0.2, color=eb.lines[0].get_color())
print(np.round(lo, 3).tolist(), np.round(hi, 3).tolist(), len(eb.lines[1]))
fig.savefig("eb_band.png", dpi=50)
