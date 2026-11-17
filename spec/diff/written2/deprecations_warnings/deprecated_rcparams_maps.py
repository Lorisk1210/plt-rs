# The deprecated/obsolete rcParams tables: set each listed key and record the warning class, then read the stored value.
import warnings
import matplotlib
matplotlib.use("agg")

dep = getattr(matplotlib, "_deprecated_map", {})
ign = getattr(matplotlib, "_deprecated_ignore_map", {})
rem = getattr(matplotlib, "_deprecated_remain_as_none", {})
print(sorted(dep), sorted(ign), sorted(rem))
for key in sorted(dep) + sorted(ign) + sorted(rem):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        try:
            value = matplotlib.rcParams[key]
            matplotlib.rcParams[key] = value
            out = repr(matplotlib.rcParams[key])[:40]
        except Exception as e:
            out = type(e).__name__
    print(key, out, sorted({w.category.__name__ for w in caught}))
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    try:
        matplotlib.rcParams["not.a.key"] = 1
    except KeyError as e:
        print("KeyError", str(e)[:70])
print(len(caught))
print(len(matplotlib.rcParams), len(matplotlib.rcParamsDefault) == len(matplotlib.rcParams))
print(sorted(k for k in matplotlib.rcParams if k.startswith("_"))[:3])
hidden = [k for k in matplotlib.rcParams.keys() if k.startswith("_internal")]
print(len(hidden))
print(len(matplotlib.rcsetup._validators) >= len(matplotlib.rcParams))
