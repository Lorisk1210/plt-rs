# Tiny measurement differences produce an offset and scientific notation; turn them off and on.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import ScalarFormatter

x = np.arange(10)
y = 1_000_000 + np.array([0.1, 0.4, 0.3, 0.8, 0.6, 1.0, 0.9, 1.3, 1.2, 1.5])
fig, axs = plt.subplots(1, 3, figsize=(10, 3))
for ax in axs:
    ax.plot(x, y, ".-")
axs[0].set_title("default")
axs[1].ticklabel_format(useOffset=False, style="plain")
axs[1].set_title("no offset")
fmt = ScalarFormatter(useMathText=True)
fmt.set_powerlimits((-2, 2))
axs[2].yaxis.set_major_formatter(fmt)
axs[2].set_title("math text")
fig.tight_layout()
fig.canvas.draw()
print(axs[0].yaxis.get_offset_text().get_text())
print(axs[2].yaxis.get_offset_text().get_text())
fig.savefig("offsets.png")
