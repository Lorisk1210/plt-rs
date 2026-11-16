# Melt a wide table to long format and draw one subplot per variable (manual faceting).
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

wide = pd.DataFrame({"month": range(1, 13), "rain": [80, 60, 55, 45, 50, 40, 30, 35, 50, 70, 85, 90],
                     "sun": [60, 80, 120, 170, 210, 230, 250, 230, 170, 120, 70, 50],
                     "temp": [2, 3, 7, 11, 15, 18, 20, 19, 16, 11, 6, 3]})
long = wide.melt(id_vars="month", var_name="var", value_name="value")
names = long["var"].unique()
fig, axs = plt.subplots(1, len(names), figsize=(10, 3), sharex=True)
for ax, name in zip(axs, names):
    sub = long[long["var"] == name]
    ax.plot(sub["month"], sub["value"], "o-")
    ax.set_title(name)
print(long.shape, list(names))
fig.tight_layout()
fig.savefig("facets.png")
