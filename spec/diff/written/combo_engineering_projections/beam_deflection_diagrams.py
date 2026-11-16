# Beam analysis: load arrows (annotations), shear and moment diagrams with filled areas, and deflection with max marker.
import numpy as np
import matplotlib.pyplot as plt

L, w = 10.0, 2.0
x = np.linspace(0, L, 200)
V = w * (L / 2 - x)
M = w * x * (L - x) / 2
EI = 1e3
defl = -w * x * (L ** 3 - 2 * L * x ** 2 + x ** 3) / (24 * EI)
fig, axs = plt.subplots(4, 1, figsize=(8, 10), sharex=True)
axs[0].plot([0, L], [0, 0], "k", lw=4)
for xi in np.linspace(0.5, 9.5, 10):
    axs[0].annotate("", xy=(xi, 0), xytext=(xi, 1), arrowprops=dict(arrowstyle="->"))
axs[0].plot([0, L], [0, 0], "^", ms=12, color="C1")
axs[0].set_ylim(-0.5, 1.5); axs[0].axis("off")
axs[1].fill_between(x, V, 0, where=V >= 0, color="C0", alpha=0.4); axs[1].fill_between(x, V, 0, where=V < 0, color="C3", alpha=0.4)
axs[1].set_ylabel("V")
axs[2].fill_between(x, M, color="C2", alpha=0.4); axs[2].set_ylabel("M")
axs[3].plot(x, defl * 1000)
k = defl.argmin()
axs[3].plot(x[k], defl[k] * 1000, "ro")
axs[3].set_ylabel("deflection [mm]")
print(round(float(M.max()), 3), round(float(defl.min() * 1000), 3))
fig.savefig("beam.png", dpi=50)
