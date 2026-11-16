# Budget vs actual: grouped bars per department, variance bars coloured by sign, and a cumulative variance line.
import numpy as np
import matplotlib.pyplot as plt

depts = ["IT", "HR", "Sales", "Ops", "R&D", "Legal"]
budget = np.array([120, 40, 200, 150, 180, 30])
actual = np.array([135, 38, 210, 140, 195, 45])
var = actual - budget
x = np.arange(len(depts))
fig, (a, b) = plt.subplots(2, 1, figsize=(9, 7), sharex=True)
a.bar(x - 0.2, budget, 0.4, label="budget", color="0.7")
a.bar(x + 0.2, actual, 0.4, label="actual", color="C0")
a.legend()
bars = b.bar(x, var, color=np.where(var > 0, "C3", "C2"))
b.bar_label(bars, fmt="%+d")
b.axhline(0, color="k", lw=0.5)
tw = b.twinx()
tw.plot(x, np.cumsum(var), "ko--")
b.set_xticks(x, depts)
print(var.tolist(), int(var.sum()))
fig.savefig("budget.png", dpi=50)
