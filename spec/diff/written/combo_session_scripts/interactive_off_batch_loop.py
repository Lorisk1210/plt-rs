# Batch mode: ioff, a loop creating, filling and saving figures with plt API, plt.close per iteration, final count.
import numpy as np
import matplotlib.pyplot as plt

plt.ioff()
rng = np.random.default_rng(243)
sizes = []
for k in range(8):
    plt.figure(figsize=(3 + k * 0.2, 2.5))
    plt.plot(rng.normal(size=50).cumsum())
    plt.fill_between(range(50), -5, 5, alpha=0.1)
    plt.title(f"run {k}")
    plt.ylim(-15, 15)
    plt.savefig(f"run_{k}.png", dpi=30)
    sizes.append(tuple(plt.gcf().get_size_inches().round(2)))
    plt.close()
print(sizes[:3], plt.get_fignums(), plt.isinteractive())
