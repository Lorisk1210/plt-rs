# A function, its numerical derivative (np.gradient) and cumulative integral on three stacked axes.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 4 * np.pi, 400)
f = np.sin(x) * np.exp(-x / 8)
df = np.gradient(f, x)
F = np.concatenate([[0], np.cumsum((f[1:] + f[:-1]) / 2 * np.diff(x))])
fig, axs = plt.subplots(3, 1, figsize=(6, 6), sharex=True)
for ax, y, name in zip(axs, [f, df, F], ["f", "f'", "∫f"]):
    ax.plot(x, y)
    ax.set_ylabel(name)
    ax.axhline(0, color="k", lw=0.5)
axs[-1].set_xlabel("x")
print(round(float(F[-1]), 5))
fig.savefig("calculus.png")
