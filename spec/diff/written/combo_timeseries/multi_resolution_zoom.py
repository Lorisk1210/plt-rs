# Overview of a long series with a highlighted window and a zoomed panel connected by ConnectionPatch lines.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.patches import ConnectionPatch
import matplotlib.dates as mdates

rng = np.random.default_rng(56)
idx = pd.date_range("2024-01-01", periods=2000, freq="h")
s = pd.Series(np.cumsum(rng.normal(0, 1, 2000)), index=idx)
w0, w1 = idx[900], idx[1100]
fig, (top, bottom) = plt.subplots(2, 1, figsize=(10, 6))
top.plot(s.index, s, lw=0.6)
top.axvspan(w0, w1, color="orange", alpha=0.3)
part = s[w0:w1]
bottom.plot(part.index, part, color="orange")
for x_top, x_bottom in [(w0, part.index[0]), (w1, part.index[-1])]:
    con = ConnectionPatch(xyA=(mdates.date2num(x_top), top.get_ylim()[0]), coordsA=top.transData,
                          xyB=(mdates.date2num(x_bottom), bottom.get_ylim()[1]), coordsB=bottom.transData, color="gray")
    fig.add_artist(con)
fig.autofmt_xdate()
print(len(part), round(float(part.max()), 4))
fig.savefig("zoom_window.png", dpi=50)
