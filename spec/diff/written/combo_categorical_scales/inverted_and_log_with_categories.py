# Ranking chart: categories on an inverted y, log-scaled x values, error bars and value labels.
import numpy as np
import matplotlib.pyplot as plt

names = ["Shanghai", "Singapore", "Rotterdam", "Busan", "Antwerp", "Hamburg"]
teu = np.array([47.3, 37.3, 14.5, 22.7, 12.0, 8.3]) * 1e6
order = np.argsort(-teu)
fig, ax = plt.subplots(figsize=(7, 4))
ax.errorbar(teu[order], [names[i] for i in order], xerr=teu[order] * 0.05, fmt="o", capsize=3)
ax.set_xscale("log")
ax.invert_yaxis()
for v, n in zip(teu[order], [names[i] for i in order]):
    ax.text(v * 1.08, n, f"{v / 1e6:.1f} M", va="center", fontsize=8)
ax.set_xlim(5e6, 1e8)
fig.canvas.draw()
print([t.get_text() for t in ax.get_yticklabels()], ax.yaxis_inverted(), ax.get_xscale())
fig.savefig("ports.png", dpi=50)
