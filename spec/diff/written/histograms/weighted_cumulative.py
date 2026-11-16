# Weighted histogram (sales weighted by amount) and a cumulative normalised histogram.
import numpy as np
import matplotlib.pyplot as plt

np.random.seed(8)
price = np.random.uniform(5, 100, 500)
qty = np.random.randint(1, 10, 500)
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
a.hist(price, bins=10, weights=qty, color="olive", rwidth=0.85)
a.set_title("units sold by price band")
b.hist(price, bins=50, cumulative=True, density=True, histtype="stepfilled", alpha=0.4)
b.hist(price, bins=50, cumulative=-1, density=True, histtype="step", color="k")
b.set_title("cumulative")
print(int(qty.sum()))
fig.tight_layout()
fig.savefig("prices.png")
