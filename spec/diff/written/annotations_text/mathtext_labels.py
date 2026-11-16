# Physics plot with mathtext in title, labels, legend and an equation placed in data coordinates.
import numpy as np
import matplotlib.pyplot as plt

r = np.linspace(0.5, 5, 200)
fig, ax = plt.subplots(figsize=(6, 4))
for n in (1, 2, 3):
    ax.plot(r, n ** 2 / r ** 2 * np.exp(-r / n), label=rf"$n={n}$: $\frac{{n^2}}{{r^2}}e^{{-r/n}}$")
ax.set_xlabel(r"radius $r$ [$\AA$]")
ax.set_ylabel(r"$\psi^2(r)\ \mathrm{[a.u.]}$")
ax.set_title(r"Toy densities, $\sum_{i=1}^{3} \alpha_i \sqrt{\beta_i}$")
ax.text(2.5, 2.0, r"$E = mc^2 \quad \Delta x \Delta p \geq \hbar/2$", fontsize=12)
ax.legend()
fig.canvas.draw()
print(ax.title.get_window_extent().width > 0)
fig.savefig("densities.png")
