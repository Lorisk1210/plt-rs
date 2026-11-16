# A script that leaves several figures open at the end (no savefig), as many scripts run in IDEs do, plus one closed.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 1, 50)
fig_a = plt.figure(figsize=(4, 3)); plt.plot(x, x ** 2); plt.title("left open A")
fig_b, ax_b = plt.subplots(figsize=(4, 3)); ax_b.bar([1, 2], [3, 1]); ax_b.set_title("left open B")
fig_c = plt.figure(figsize=(4, 3)); plt.plot(x, np.sqrt(x)); plt.close(fig_c)
fig_d, axs = plt.subplots(1, 2, figsize=(6, 3)); axs[0].pie([1, 1]); axs[1].scatter(x, x)
print(plt.get_fignums(), [f.axes[0].get_title() for f in map(plt.figure, plt.get_fignums()) if f.axes])
fig_d.suptitle("left open D")
print(len(fig_d.axes), fig_b.get_size_inches().tolist())
fig_a.axes[0].set_xlabel("x")
