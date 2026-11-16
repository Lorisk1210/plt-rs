# Pickle a styled figure, load two copies, modify each differently (theme, data) and export both plus the original.
import pickle
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot(np.arange(10), np.arange(10) ** 1.5, "o-", label="growth")
ax.set_title("base")
ax.legend()
blob = pickle.dumps(fig)
dark = pickle.loads(blob)
dark.set_facecolor("#222")
dax = dark.axes[0]
dax.set_facecolor("#333")
dax.set_title("dark copy", color="w")
dax.tick_params(colors="w")
data = pickle.loads(blob)
data.axes[0].lines[0].set_ydata(np.arange(10) ** 1.2)
data.axes[0].set_title("new data")
for f, name in [(fig, "base"), (dark, "dark"), (data, "data")]:
    f.savefig(f"{name}.png", dpi=50, facecolor=f.get_facecolor())
print(fig.axes[0].get_title(), dark.axes[0].get_title(), np.round(data.axes[0].lines[0].get_ydata()[-1], 4))
