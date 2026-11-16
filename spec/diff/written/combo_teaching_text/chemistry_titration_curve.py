# Titration curve: pH vs volume with equivalence point annotation, derivative on a twin axis, indicator colour band.
import numpy as np
import matplotlib.pyplot as plt

V = np.linspace(0, 50, 500)
Ca, Va, Cb = 0.1, 25, 0.1
nOH = Cb * V / 1000 - Ca * Va / 1000
H = np.where(nOH < 0, -nOH / ((Va + V) / 1000), 1e-14 / np.maximum(nOH / ((Va + V) / 1000), 1e-14))
pH = -np.log10(np.clip(H, 1e-14, 1))
dpH = np.gradient(pH, V)
eq = V[dpH.argmax()]
fig, ax = plt.subplots(figsize=(8, 5))
ax.plot(V, pH, "k")
ax.axhspan(8.2, 10, color="magenta", alpha=0.15, label="phenolphthalein")
ax.annotate(f"equivalence\n{eq:.1f} mL", xy=(eq, pH[dpH.argmax()]), xytext=(eq + 5, 5), arrowprops=dict(arrowstyle="->"))
tw = ax.twinx()
tw.plot(V, dpH, color="C0", alpha=0.6)
tw.set_ylabel("dpH/dV", color="C0")
ax.set(xlabel="NaOH added [mL]", ylabel="pH")
ax.legend(loc="upper left")
print(round(float(eq), 2), round(float(pH[0]), 3), round(float(pH[-1]), 3))
fig.savefig("titration.png", dpi=50)
