# Old configuration idioms: matplotlib.rc(group, **kw) groups and aliases, rcParams dict assignments, and rcdefaults afterwards.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

matplotlib.rc("font", size=8, family="monospace")
matplotlib.rc("lines", lw=3, ls="--", c="k", mew=2, aa=False)
matplotlib.rc("lines", markersize=4)
matplotlib.rc("axes", fc="0.9", ec="r", lw=2)
matplotlib.rc(("xtick", "ytick"), color="g", labelsize="small")
print(plt.rcParams["font.size"], plt.rcParams["font.family"], plt.rcParams["lines.linewidth"], plt.rcParams["lines.linestyle"])
print(plt.rcParams["lines.color"], plt.rcParams["lines.markeredgewidth"], plt.rcParams["lines.antialiased"])
print(plt.rcParams["axes.facecolor"], plt.rcParams["axes.edgecolor"], plt.rcParams["axes.linewidth"])
print(plt.rcParams["xtick.color"], plt.rcParams["ytick.labelsize"])
fig, ax = plt.subplots()
ln, = ax.plot([0, 1])
print(ln.get_linewidth(), ln.get_linestyle(), ln.get_antialiased(), ax.get_facecolor())
fig.savefig("old_rc.png")
font = {"family": "serif", "weight": "bold", "size": 11}
matplotlib.rc("font", **font)
print(plt.rcParams["font.weight"], plt.rcParams["font.family"])
try:
    matplotlib.rc("lines", bogus=1)
except KeyError as e:
    print("KeyError", str(e)[:60])
matplotlib.rcdefaults()
print(plt.rcParams["font.size"], plt.rcParams["lines.linewidth"], plt.rcParams["axes.facecolor"])
print(sorted(k for k in matplotlib.rcParams if k.startswith("lines."))[:5])
print(matplotlib.rcParams["axes.prop_cycle"].by_key()["color"][:3])
print(matplotlib.__version__ >= "3", tuple(matplotlib.__version_info__)[:1] >= (3,))
print(plt.rcParams["lines.markersize"], plt.rcParams["xtick.color"])
