# pandas registers its own converters for Period and Timestamp; plot pd.Period values via matplotlib.
import pandas as pd
import matplotlib.pyplot as plt
from pandas.plotting import register_matplotlib_converters

register_matplotlib_converters()
periods = pd.period_range("2024-01", periods=6, freq="M")
fig, ax = plt.subplots(figsize=(6, 3))
ax.plot(periods.to_timestamp(), [5, 6, 4, 7, 8, 6], "o-")
ax.set_title("periods via timestamps")
fig.autofmt_xdate()
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()][:3])
fig.savefig("periods.png")
