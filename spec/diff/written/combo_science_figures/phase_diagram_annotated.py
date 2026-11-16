# Phase diagram: filled regions by contourf with labels, phase boundary lines, a critical point annotation and log pressure.
import numpy as np
import matplotlib.pyplot as plt

T = np.linspace(200, 700, 300)
P = np.logspace(-2, 3, 300)
TT, PP = np.meshgrid(T, P)
boundary1 = 273 + 10 * np.log10(PP)
boundary2 = 373 + 60 * np.log10(PP)
phase = np.where(TT < boundary1, 0, np.where(TT < boundary2, 1, 2))
phase[(TT > 647) & (PP > 220)] = 3
fig, ax = plt.subplots(figsize=(7, 5))
ax.contourf(TT, PP, phase, levels=[-0.5, 0.5, 1.5, 2.5, 3.5], colors=["#cfe8ff", "#7fb8e6", "#f6e0b5", "#e8b4d0"])
ax.contour(TT, PP, phase, levels=[0.5, 1.5], colors="k", linewidths=1.5)
ax.set_yscale("log")
for (tx, py, name) in [(230, 10, "solid"), (330, 100, "liquid"), (550, 0.05, "gas"), (670, 600, "supercritical")]:
    ax.text(tx, py, name, ha="center", fontsize=11)
ax.annotate("critical point", xy=(647, 220), xytext=(450, 600), arrowprops=dict(arrowstyle="->"))
ax.plot(647, 220, "ko")
ax.set_xlabel("T [K]")
ax.set_ylabel("p [bar]")
print(np.bincount(phase.ravel()).tolist())
fig.savefig("phase.png", dpi=50)
