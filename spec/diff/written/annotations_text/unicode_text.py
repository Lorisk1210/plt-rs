# Labels in several scripts and symbols (°, µ, ±, Greek, arrows) that DejaVu Sans covers.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
x = np.linspace(0, 1, 30)
ax.plot(x, 25 + 3 * x, label="Temperatur (°C) ± 0.5")
ax.plot(x, 24 + 2 * x ** 2, label="température µ-sensor")
ax.set_title("Messung — mesure — μέτρηση")
ax.set_xlabel("Zeit → t / s")
ax.set_ylabel("Θ [°C]")
ax.text(0.1, 26.5, "Ωmega ≈ 3 · 10⁻³ ✓")
ax.legend()
fig.canvas.draw()
print([t.get_text() for t in ax.get_legend().get_texts()])
fig.savefig("unicode.png")
