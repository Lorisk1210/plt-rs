# datetime64 inputs of many units (ns, us, s, D, M, Y) on axes; date2num precision, limits, and the converter's behaviour.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

units = ["ns", "us", "ms", "s", "m", "h", "D", "W", "M", "Y"]
for u in units:
    first = np.datetime64("2024-01-01").astype(f"datetime64[{u}]")
    t = first + np.arange(5).astype(f"timedelta64[{u}]")
    nums = mdates.date2num(t)
    print(u, len(t), [f"{v:.9f}" for v in nums[:2]])
fig, ax = plt.subplots(figsize=(6, 2), dpi=60)
t = np.datetime64("2024-01-01T00:00:00.000000001") + np.arange(5).astype("timedelta64[ns]") * 250
ax.plot(t, range(5))
fig.canvas.draw()
print([f"{v:.12f}" for v in ax.get_xlim()], [t.get_text() for t in ax.get_xticklabels()][:3])
big = np.array(["1900-01-01", "2100-12-31"], dtype="datetime64[D]")
print(np.round(mdates.date2num(big), 3).tolist(), [d.isoformat() for d in mdates.num2date(mdates.date2num(big))])
print(mdates.date2num(np.datetime64("NaT")), mdates.date2num(np.array(["NaT"], dtype="datetime64[s]")).tolist())
print(mdates.date2num(np.datetime64("1970-01-01T00:00:00")), mdates.get_epoch())
td = np.array([1, 2], dtype="timedelta64[h]")
try:
    ax.plot(td, [1, 2])
    print("timedelta plotted", type(ax.xaxis.get_converter()).__name__)
except Exception as e:
    print(type(e).__name__, str(e)[:60])
fig.savefig("dt64.png")
print(mdates.num2date(mdates.date2num(np.datetime64("2024-02-29T12:00"))).isoformat())
