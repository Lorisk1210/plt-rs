# Materials: stress-strain curve with yield and UTS annotations, an inset of the elastic region, and Mohr's circle.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Circle

eps = np.linspace(0, 0.2, 400)
E, sy = 200e3, 250
sig = np.where(eps < sy / E, E * eps, sy + 300 * (1 - np.exp(-(eps - sy / E) * 20)) - 2000 * np.clip(eps - 0.15, 0, None) ** 2)
fig, (a, b) = plt.subplots(1, 2, figsize=(12, 4.5))
a.plot(eps * 100, sig)
i = sig.argmax()
a.annotate("UTS", (eps[i] * 100, sig[i]), xytext=(10, -40), textcoords="offset points", arrowprops=dict(arrowstyle="->"))
a.annotate("yield", (sy / E * 100, sy), xytext=(30, -30), textcoords="offset points", arrowprops=dict(arrowstyle="->"))
ins = a.inset_axes([0.5, 0.15, 0.4, 0.4]); ins.plot(eps[:20] * 100, sig[:20]); ins.set_title("elastic", fontsize=8)
sx, syy, txy = 80, -40, 30
c, r = (sx + syy) / 2, np.hypot((sx - syy) / 2, txy)
b.add_patch(Circle((c, 0), r, fill=False))
b.plot([sx, syy], [txy, -txy], "o-")
b.plot([c - r, c + r], [0, 0], "rs")
b.set_aspect("equal"); b.axhline(0, color="k", lw=0.5); b.axvline(0, color="k", lw=0.5)
b.set_xlim(c - r * 1.3, c + r * 1.3); b.set_ylim(-r * 1.3, r * 1.3)
print(round(float(sig.max()), 2), round(float(c + r), 3), round(float(c - r), 3))
fig.savefig("materials.png", dpi=50)
