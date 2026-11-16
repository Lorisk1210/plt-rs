# Cases on a log scale and growth rate on a linear twin axis, both sharing a date-like x.
import numpy as np
import matplotlib.pyplot as plt

day = np.arange(60)
cases = 10 * np.exp(0.12 * day) / (1 + np.exp(0.12 * (day - 40)))
rate = np.gradient(np.log(cases))
fig, ax = plt.subplots(figsize=(7, 4))
ax.semilogy(day, cases, color="C0")
ax.set_ylabel("cases (log)", color="C0")
ax_r = ax.twinx()
ax_r.plot(day, rate * 100, color="C1", ls="--")
ax_r.set_ylabel("daily growth %", color="C1")
ax_r.axhline(0, color="C1", lw=0.5)
ax.set_xlabel("day")
print(ax.get_yscale(), ax_r.get_yscale())
fig.savefig("epidemic.png")
