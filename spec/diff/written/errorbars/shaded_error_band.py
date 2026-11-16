# Mean of repeated runs with a ±1 sd shaded band, a common alternative to error bars.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(5)
epochs = np.arange(1, 51)
runs = np.array([1 / np.sqrt(epochs) + rng.normal(0, 0.03, 50) for _ in range(10)])
mean = runs.mean(axis=0)
sd = runs.std(axis=0)
fig, ax = plt.subplots(figsize=(6.5, 4))
ax.plot(epochs, mean, color="C0", label="mean loss")
ax.fill_between(epochs, mean - sd, mean + sd, color="C0", alpha=0.25, label="±1 sd", lw=0)
ax.set_xlabel("epoch")
ax.set_ylabel("loss")
ax.set_yscale("log")
ax.legend()
print(np.round(mean[[0, -1]], 4))
fig.savefig("loss.png")
