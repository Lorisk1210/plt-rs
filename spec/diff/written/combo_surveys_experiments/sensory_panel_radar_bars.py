# Product tasting panel: radar of mean attribute scores per product, grouped bars, and a ranking with medals.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(78)
attrs = ["sweet", "sour", "bitter", "aroma", "body", "finish"]
products = {"A": rng.uniform(4, 8, 6), "B": rng.uniform(3, 9, 6), "C": rng.uniform(5, 7, 6)}
ang = np.linspace(0, 2 * np.pi, 6, endpoint=False)
fig = plt.figure(figsize=(12, 4.5))
r = fig.add_subplot(1, 3, 1, projection="polar")
for name, v in products.items():
    r.plot(np.r_[ang, ang[:1]], np.r_[v, v[:1]], label=name)
    r.fill(np.r_[ang, ang[:1]], np.r_[v, v[:1]], alpha=0.1)
r.set_xticks(ang, attrs)
r.legend(loc="upper right", bbox_to_anchor=(1.3, 1.1))
b = fig.add_subplot(1, 3, 2)
x = np.arange(6)
for k, (name, v) in enumerate(products.items()):
    b.bar(x + (k - 1) * 0.27, v, 0.27, label=name, color=r.lines[k].get_color())
b.set_xticks(x, attrs, rotation=45)
c = fig.add_subplot(1, 3, 3)
ranking = sorted(products, key=lambda n: -products[n].mean())
c.barh(ranking[::-1], [products[n].mean() for n in ranking[::-1]], color=["#cd7f32", "silver", "gold"])
fig.tight_layout()
print(ranking, [round(float(products[n].mean()), 3) for n in ranking])
fig.savefig("tasting.png", dpi=50)
