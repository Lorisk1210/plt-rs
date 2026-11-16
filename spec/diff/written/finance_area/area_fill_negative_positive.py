# Area chart of a balance crossing zero, filled green above and red below with interpolation.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 12, 300)
balance = 3 * np.sin(x) + 0.5 * x - 2
fig, ax = plt.subplots(figsize=(7, 3.5))
ax.plot(x, balance, color="k", lw=1)
ax.fill_between(x, balance, 0, where=balance >= 0, interpolate=True, color="green", alpha=0.4)
ax.fill_between(x, balance, 0, where=balance < 0, interpolate=True, color="red", alpha=0.4)
ax.axhline(0, color="k", lw=0.5)
ax.set_xlabel("month")
ax.set_ylabel("balance (k)")
print(round(float(np.trapezoid(np.clip(balance, 0, None), x)), 4))
fig.savefig("balance_area.png")
