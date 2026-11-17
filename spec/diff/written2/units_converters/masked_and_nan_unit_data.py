# Unit data containing masked values, NaT, None and NaN: what plot, scatter and bar draw and the limits they set.
import datetime
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 2, figsize=(7, 5))
d = np.array(["2024-01-01", "NaT", "2024-01-03", "2024-01-04"], dtype="datetime64[D]")
l1, = axs[0, 0].plot(d, [1, 2, np.nan, 4], "o-")
m = np.ma.masked_array([datetime.datetime(2024, 1, k) for k in range(1, 5)], mask=[0, 1, 0, 0])
l2, = axs[0, 1].plot(m, [1, 2, 3, 4], "s-")
sc = axs[1, 0].scatter(["a", "b", "c"], np.ma.masked_array([1.0, 2.0, 3.0], [0, 1, 0]))
bars = axs[1, 1].bar(["x", "y", "z"], [1, np.nan, 3])
fig.canvas.draw()
for a in axs.flat:
    print([round(v, 3) for v in a.get_xlim()], [round(v, 3) for v in a.get_ylim()])
print(np.round(l1.get_xydata(), 3).tolist())
print(np.ma.getmaskarray(l2.get_xdata(orig=False)).tolist() if np.ma.isMaskedArray(l2.get_xdata(orig=False)) else
      np.round(np.asarray(l2.get_xdata(orig=False), float), 3).tolist())
print(sc.get_offsets().mask.tolist() if np.ma.isMaskedArray(sc.get_offsets()) else sc.get_offsets().tolist())
print([b.get_height() for b in bars])
fig.savefig("masked_units.png")
try:
    plt.figure().add_subplot().plot([None, datetime.date(2024, 1, 1)], [1, 2])
    print("None accepted")
except Exception as e:
    print(type(e).__name__, str(e)[:60])
ax = plt.figure().add_subplot()
ax.plot(np.array(["NaT", "NaT"], dtype="datetime64[D]"), [1, 2])
print([round(v, 3) for v in ax.get_xlim()])
