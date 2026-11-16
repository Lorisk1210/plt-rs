# Inventory: stock levels per SKU with reorder points (step lines), days of cover bars coloured by risk, and ABC Pareto.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(212)
days = np.arange(60)
fig = plt.figure(figsize=(12, 7))
a = fig.add_subplot(2, 1, 1)
covers = {}
for k, sku in enumerate(["A12", "B07", "C33"]):
    stock, level = [], 200
    for d in days:
        level -= rng.poisson(8 + k * 3)
        if level < 60:
            level += 200
        stock.append(level)
    line, = a.step(days, stock, where="post", label=sku)
    a.axhline(60, color=line.get_color(), ls=":", lw=0.8)
    covers[sku] = stock[-1] / (8 + k * 3)
a.legend()
b = fig.add_subplot(2, 2, 3)
b.bar(list(covers), list(covers.values()), color=["C3" if v < 7 else "C2" for v in covers.values()])
c = fig.add_subplot(2, 2, 4)
value = np.sort(rng.pareto(1.5, 50) * 100)[::-1]
c.bar(range(50), value, color="0.6")
c2 = c.twinx(); c2.plot(np.cumsum(value) / value.sum() * 100, "r")
c2.axhline(80, ls="--", color="k")
fig.tight_layout()
print({k: round(v, 2) for k, v in covers.items()}, int(np.searchsorted(np.cumsum(value) / value.sum(), 0.8)))
fig.savefig("inventory.png", dpi=50)
