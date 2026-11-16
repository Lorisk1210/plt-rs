# Non-uniform grid (log-spaced y) heat map of a spectrogram-like array with pcolormesh.
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(0, 10, 101)
f = np.logspace(1, 3, 41)
T, F = np.meshgrid(t, f)
power = np.exp(-((np.log10(F) - 1.5 - 0.1 * T) ** 2) / 0.05)
fig, ax = plt.subplots(figsize=(7, 4))
mesh = ax.pcolormesh(T, F, power[:-1, :-1], cmap="inferno", shading="flat")
ax.set_yscale("log")
ax.set_xlabel("time [s]")
ax.set_ylabel("frequency [Hz]")
fig.colorbar(mesh, ax=ax, label="power")
print(mesh.get_array().shape)
fig.savefig("spectrogram.png")
