# Dates given as pandas Timestamps, numpy datetime64 and datetime objects on one axis.
import datetime as dt
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(7, 3.5))
ax.plot([pd.Timestamp("2024-01-01"), pd.Timestamp("2024-01-10")], [1, 2], "o-", label="Timestamp")
ax.plot(np.array(["2024-01-03", "2024-01-12"], dtype="datetime64[D]"), [2, 3], "s-", label="datetime64")
ax.plot([dt.datetime(2024, 1, 5, 12), dt.datetime(2024, 1, 15)], [1.5, 1], "^-", label="datetime")
ax.legend()
fig.autofmt_xdate()
print([round(v, 3) for v in ax.get_xlim()])
fig.savefig("mixed_dates.png")
