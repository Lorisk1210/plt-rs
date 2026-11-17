# Nested rc_context, rc() calls inside them, an exception unwinding a context, and rcdefaults / rc_file_defaults restoring state.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

base = dict(plt.rcParams)
print(plt.rcParams["lines.linewidth"], plt.rcParams["axes.titlesize"])
with plt.rc_context({"lines.linewidth": 3}):
    plt.rc("axes", titlesize=20, labelsize="small")
    with plt.rc_context({"lines.linewidth": 5, "axes.titlesize": "x-large"}):
        print(plt.rcParams["lines.linewidth"], plt.rcParams["axes.titlesize"], plt.rcParams["axes.labelsize"])
        try:
            with plt.rc_context({"lines.linewidth": 9}):
                raise KeyError("inner")
        except KeyError:
            pass
        print(plt.rcParams["lines.linewidth"])
    print(plt.rcParams["lines.linewidth"], plt.rcParams["axes.titlesize"], plt.rcParams["axes.labelsize"])
print(plt.rcParams["lines.linewidth"], plt.rcParams["axes.titlesize"], plt.rcParams["axes.labelsize"])
changed = sorted(k for k in base if base[k] != plt.rcParams[k])
print(changed)
plt.rc("lines", lw=4, c="r")
plt.rc(("xtick", "ytick"), labelsize=6)
print(plt.rcParams["lines.linewidth"], plt.rcParams["lines.color"], plt.rcParams["xtick.labelsize"], plt.rcParams["ytick.labelsize"])
plt.rcdefaults()
print(plt.rcParams["lines.linewidth"], plt.rcParams["lines.color"], plt.rcParams["xtick.labelsize"])
plt.rcParams["figure.dpi"] = 50
matplotlib.rc_file_defaults()
print(plt.rcParams["figure.dpi"], plt.rcParams["backend"] in ("agg", "Agg"))
with plt.rc_context(fname=None, rc={"font.family": "serif"}):
    print(plt.rcParams["font.family"])
print(plt.rcParams["font.family"])
try:
    plt.rc("lines", nonsense=1)
except KeyError as e:
    print("KeyError", str(e)[:60])
