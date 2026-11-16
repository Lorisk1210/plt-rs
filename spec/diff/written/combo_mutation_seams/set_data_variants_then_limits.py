# Update line data in different ways (set_data, set_xdata/ydata, lists, arrays, pandas) then relim and print limits.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
line, = ax.plot([0, 1], [0, 1])
steps = []
line.set_data([0, 1, 2, 3], [1, 3, 2, 4]); ax.relim(); ax.autoscale_view(); steps.append(ax.get_ylim())
line.set_ydata(np.array([10, 20, 15, 25])); ax.relim(); ax.autoscale_view(); steps.append(ax.get_ylim())
line.set_xdata(pd.Series([5, 6, 7, 8])); ax.relim(); ax.autoscale_view(); steps.append(ax.get_xlim())
line.set_data(np.linspace(0, 1, 5), [np.nan, 1, 2, np.nan, 3]); ax.relim(); ax.autoscale_view(); steps.append(ax.get_ylim())
for s in steps:
    print([round(float(v), 4) for v in s])
print(type(line.get_xdata()).__name__, type(line.get_ydata()).__name__, line.get_ydata())
fig.savefig("set_data.png", dpi=50)
