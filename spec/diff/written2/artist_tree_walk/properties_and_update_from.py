# Artist.properties() dicts compared between artists, update_from copying style, set() with aliases and conflicting kwargs.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Circle, Rectangle

fig, ax = plt.subplots()
a, = ax.plot([0, 1], [0, 1], color="r", lw=3, ls="--", marker="o", alpha=0.5, zorder=4)
b, = ax.plot([0, 1], [1, 0])
pa, pb = a.properties(), b.properties()
diff = sorted(k for k in pa if k in pb and str(pa[k]) != str(pb[k]))
print(diff)
b.update_from(a)
pb2 = b.properties()
print(sorted(k for k in pa if str(pa[k]) != str(pb2[k])))
c = Circle((0, 0), 1, fc="y", ec="k", lw=2, hatch="//")
r = Rectangle((0, 0), 1, 1)
r.update_from(c)
print(r.get_facecolor(), r.get_edgecolor(), r.get_linewidth(), r.get_hatch())
ln, = ax.plot([0, 1])
ln.set(lw=5, c="m", ls=":", mfc="w")
print(ln.get_linewidth(), ln.get_color(), ln.get_linestyle(), ln.get_markerfacecolor())
try:
    ln.set(lw=1, linewidth=2)
except TypeError as e:
    print("TypeError", str(e)[:70])
try:
    ln.set(nonexistent=1)
except AttributeError as e:
    print("AttributeError", str(e)[:70])
props = ax.properties()
print(len(props) > 50, sorted(props)[:6])
print(sorted(k for k in ln.properties() if "data" in k))
tp = ax.title.properties()
print(sorted(k for k in tp if k.startswith("font"))[:6], tp["text"] == "")
print(type(a.properties()["path"]).__name__, a.properties()["children"])
