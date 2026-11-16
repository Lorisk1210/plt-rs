# One subplot per column with subplots=True, layout and sharex, from a wide DataFrame.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

t = pd.date_range("2024-01-01", periods=100, freq="D")
df = pd.DataFrame({f"sensor {k}": np.sin(np.arange(100) / (5 + k)) + k for k in range(6)}, index=t)
axes = df.plot(subplots=True, layout=(3, 2), figsize=(9, 6), sharex=True, legend=False, title=list(df.columns))
plt.tight_layout()
print(axes.shape)
plt.savefig("sensors.png")
current = plt.gcf()
print(len(current.axes), current.get_size_inches())
current.savefig("df_plot_subplots_layout_small.png", dpi=50)
