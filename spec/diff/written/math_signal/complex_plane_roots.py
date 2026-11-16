# Roots of unity and a polynomial's roots on the complex plane with the unit circle.
import numpy as np
import matplotlib.pyplot as plt

n = 7
roots = np.exp(2j * np.pi * np.arange(n) / n)
poly_roots = np.roots([1, -1, 0.5, 0.3, -0.2])
fig, ax = plt.subplots(figsize=(5, 5))
ax.add_patch(plt.Circle((0, 0), 1, fill=False, ls="--", color="gray"))
ax.scatter(roots.real, roots.imag, s=60, label=f"{n}th roots of unity")
ax.scatter(poly_roots.real, poly_roots.imag, marker="x", s=80, color="r", label="polynomial roots")
ax.axhline(0, color="k", lw=0.5)
ax.axvline(0, color="k", lw=0.5)
ax.set_aspect("equal")
ax.legend(loc="upper right", fontsize=8)
print(np.round(np.sort_complex(poly_roots), 4))
fig.savefig("roots.png")
