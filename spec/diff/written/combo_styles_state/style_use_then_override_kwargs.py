# style.use('ggplot') then explicit keyword overrides; read back which property came from where.
import numpy as np
import matplotlib.pyplot as plt

plt.style.use("ggplot")
fig, axs = plt.subplots(1, 3, figsize=(11, 3.5))
l1, = axs[0].plot(np.arange(5))
l2, = axs[0].plot(np.arange(5)[::-1], color="k", lw=4)
b = axs[1].bar(range(3), [1, 2, 3], edgecolor="red")[0]
h = axs[2].hist(np.random.default_rng(91).normal(size=200), bins=10, color="purple")[2][0]
axs[2].grid(False)
print(l1.get_color(), l1.get_linewidth(), l2.get_color(), l2.get_linewidth())
print(b.get_facecolor(), b.get_edgecolor(), h.get_facecolor())
print(axs[2].xaxis._major_tick_kw.get("gridOn"), axs[0].get_facecolor())
fig.savefig("ggplot_overrides.png", dpi=50)
