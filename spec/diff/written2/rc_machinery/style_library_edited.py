# Edit plt.style.library in place, register a style by name through USER_LIBRARY_PATHS-like reload, and list available styles.
import os
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import style

print(len(style.available) > 20, "ggplot" in style.available, "classic" in style.available)
print(sorted(s for s in style.available if s.startswith("seaborn-v0_8"))[:3])
style.library["mine"] = matplotlib.RcParams({"axes.facecolor": "0.9", "lines.linewidth": 4})
style.available.append("mine")
with style.context("mine"):
    print(plt.rcParams["axes.facecolor"], plt.rcParams["lines.linewidth"])
os.makedirs("mystyles", exist_ok=True)
with open("mystyles/dots.mplstyle", "w") as f:
    f.write("lines.marker: o\nlines.markersize: 3\n")
style.USER_LIBRARY_PATHS.append(os.path.abspath("mystyles"))
style.reload_library()
print("dots" in style.available, "mine" in style.available)
with style.context("dots"):
    fig, ax = plt.subplots()
    ln, = ax.plot([0, 1, 2])
    print(ln.get_marker(), ln.get_markersize())
    fig.savefig("dots.png")
style.USER_LIBRARY_PATHS.remove(os.path.abspath("mystyles"))
style.reload_library()
print("dots" in style.available)
ggplot = style.library["ggplot"]
print(type(ggplot).__name__, ggplot["axes.facecolor"], len(ggplot) > 10)
with style.context("default"):
    print(plt.rcParams["axes.facecolor"])
with style.context(["classic", "dark_background"], after_reset=True):
    print(plt.rcParams["axes.facecolor"], plt.rcParams["lines.linewidth"])
try:
    style.use("no-such-style-name")
except OSError as e:
    print("OSError", str(e).splitlines()[0][:70])
print(plt.rcParams["axes.facecolor"])
