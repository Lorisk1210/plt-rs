# StrCategoryConverter and UnitData used directly: update order, duplicate handling, numbers as strings, and the locator/formatter.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import category

ud = category.UnitData(["b", "a", "c", "a"])
print(dict(ud._mapping))
ud.update(["d", "b", "e"])
print(list(ud._mapping.items()))
try:
    ud.update([1, 2])
except TypeError as e:
    print("TypeError", str(e)[:60])
conv = category.StrCategoryConverter()
print(conv.convert(["a", "e", "b"], ud, None).tolist(), conv.convert("c", ud, None))
try:
    conv.convert(["zzz"], ud, None)
except Exception as e:
    print(type(e).__name__)
info = conv.axisinfo(ud, None)
print(type(info.majloc).__name__, type(info.majfmt).__name__, info.majloc(), info.majfmt(1, 1), info.majfmt.format_ticks([0, 1]))
fig, ax = plt.subplots()
ax.plot(["10", "2", "1"], [1, 2, 3])
ax.plot(["2", "30"], [0, 0])
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()], dict(ax.xaxis.get_units()._mapping))
ax.xaxis.set_units(category.UnitData(["30", "2", "10", "1"]))
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()], np.round(ax.lines[0].get_xdata(orig=False), 2).tolist())
fig2, ax2 = plt.subplots()
ax2.scatter(np.array([b"x", b"y"]), [1, 2])
fig2.canvas.draw()
print([t.get_text() for t in ax2.get_xticklabels()])
fig.savefig("categories.png")
