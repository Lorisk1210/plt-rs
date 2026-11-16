# Heat exchanger: hot/cold stream temperature profiles with LMTD annotation, fill between streams, and a pinch marker.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 1, 100)
Th = 150 - 70 * (1 - np.exp(-2 * x)) / (1 - np.exp(-2))
Tc = 30 + 60 * x ** 0.9
dT = Th - Tc
lmtd = (dT[0] - dT[-1]) / np.log(dT[0] / dT[-1])
fig, ax = plt.subplots(figsize=(8, 5))
ax.plot(x, Th, "C3", label="hot stream")
ax.plot(x, Tc, "C0", label="cold stream")
ax.fill_between(x, Tc, Th, color="orange", alpha=0.15)
k = dT.argmin()
ax.annotate(f"pinch ΔT = {dT[k]:.1f} K", xy=(x[k], (Th[k] + Tc[k]) / 2), xytext=(0.3, 60), arrowprops=dict(arrowstyle="->"))
ax.text(0.05, 140, f"LMTD = {lmtd:.1f} K", bbox=dict(fc="w"))
ax.set(xlabel="position", ylabel="T [°C]")
ax.legend(loc="center right")
print(round(float(lmtd), 3), round(float(dT.min()), 3))
fig.savefig("exchanger.png", dpi=50)
