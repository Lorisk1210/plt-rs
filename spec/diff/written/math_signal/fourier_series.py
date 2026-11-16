# Fourier partial sums approximating a square wave, showing the Gibbs overshoot.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-np.pi, np.pi, 1000)
square = np.sign(np.sin(x))
fig, ax = plt.subplots(figsize=(7, 4))
ax.plot(x, square, "k", lw=1, label="square")
for n in (1, 3, 9, 49):
    partial = sum(4 / (np.pi * k) * np.sin(k * x) for k in range(1, n + 1, 2))
    ax.plot(x, partial, label=f"n={n}")
ax.legend(ncols=5, fontsize=8, loc="lower center")
ax.set_ylim(-1.5, 1.5)
overshoot = max(sum(4 / (np.pi * k) * np.sin(k * x) for k in range(1, 50, 2)))
print(round(float(overshoot), 4))
fig.savefig("fourier.png")
