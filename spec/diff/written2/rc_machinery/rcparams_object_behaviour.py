# RcParams as a mapping: copies, find_all, setdefault-like access, deprecated keys, _get, iteration order and repr fragments.
import re
import matplotlib
matplotlib.use("agg")
from matplotlib import RcParams, rcParams, rcParamsDefault

c = rcParams.copy()
print(type(c).__name__, len(c) == len(rcParams), c is rcParams)
c["lines.linewidth"] = 7
print(rcParams["lines.linewidth"] != 7)
found = rcParams.find_all(r"^xtick\.major\.")
print(type(found).__name__, sorted(found))
print(list(rcParams)[:3] == sorted(rcParams)[:3])
fresh = RcParams({"lines.linewidth": "3", "axes.grid": "yes"})
print(fresh["lines.linewidth"], fresh["axes.grid"], len(fresh))
try:
    fresh["lines.linewidth"] = "thick"
except ValueError as e:
    print("ValueError", str(e)[:70])
try:
    RcParams({"no.such.key": 1})
except KeyError as e:
    print("KeyError", str(e)[:70])
text = repr(RcParams({"lines.linewidth": 2.0, "axes.grid": True}))
print(text.splitlines()[0], len(text.splitlines()))
print(str(RcParams({"lines.color": "r"})))
print(rcParamsDefault["lines.linewidth"], rcParamsDefault["figure.figsize"])
print(rcParams._get("backend") is not None)
d = dict(rcParams)
print(isinstance(d["axes.prop_cycle"], type(rcParams["axes.prop_cycle"])))
sub = {k: v for k, v in rcParams.items() if k.startswith("legend.")}
print(len(sub), sorted(sub)[:4])
c.update({"axes.titlesize": "small", "axes.labelsize": 9})
print(c["axes.titlesize"], c["axes.labelsize"])
print(sorted(re.sub(r"\..*", "", k) for k in rcParams if "." in k)[-1])
