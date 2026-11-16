# Missing dates (NaT) in a pandas series and how they show as gaps on the date axis.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

dates = pd.to_datetime(["2024-01-01", "2024-01-02", None, "2024-01-04", "2024-01-05"])
vals = np.array([1.0, 2.0, 3.0, 2.5, 4.0])
fig, ax = plt.subplots(figsize=(6, 3))
ax.plot(dates, vals, "o-")
fig.autofmt_xdate()
print(int(dates.isna().sum()), [round(v, 3) for v in ax.get_xlim()])
mask = ~dates.isna()
ax.plot(dates[mask], vals[mask] + 1, "s--")
fig.savefig("nat.png")
