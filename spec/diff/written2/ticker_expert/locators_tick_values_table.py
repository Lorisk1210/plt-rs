# tick_values of every Locator class over a set of view intervals, printed as a table (no axes needed for most).
import numpy as np
from matplotlib import ticker

intervals = [(0, 1), (-3.7, 12.2), (1e-5, 3e-5), (0.001, 1000), (-1000, -999.5)]
locators = {
    "MaxNLocator": ticker.MaxNLocator(5),
    "MaxN steps": ticker.MaxNLocator(nbins=4, steps=[1, 2.5, 5, 10], integer=False),
    "MaxN int": ticker.MaxNLocator(integer=True),
    "AutoLocator": ticker.AutoLocator(),
    "MultipleLocator": ticker.MultipleLocator(0.75, offset=0.1),
    "LinearLocator": ticker.LinearLocator(4),
    "LogLocator": ticker.LogLocator(base=10, numticks=6),
    "LogLocator subs": ticker.LogLocator(subs=(1.0, 2.0, 5.0)),
    "SymmetricalLog": ticker.SymmetricalLogLocator(base=10, linthresh=1),
    "AsinhLocator": ticker.AsinhLocator(linear_width=1),
    "LogitLocator": ticker.LogitLocator(),
    "FixedLocator": ticker.FixedLocator([0, 0.5, 2, 7, 100], nbins=3),
    "IndexLocator": ticker.IndexLocator(base=2, offset=0.5),
}
for name, loc in locators.items():
    row = []
    for lo, hi in intervals:
        try:
            if name == "IndexLocator":
                vals = [lo + 0.5 + 2 * k for k in range(3)]
            else:
                with np.errstate(all="ignore"):
                    vals = loc.tick_values(lo, hi)
            vals = np.asarray(vals, float)
            row.append((len(vals), [float(f"{v:.6g}") for v in vals[:3]]))
        except Exception as e:
            row.append(type(e).__name__)
    print(name, row)
