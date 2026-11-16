# Empirical CDFs of two samples with Axes.ecdf and a complementary version.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(2)
a = rng.exponential(1.0, 200)
b = rng.exponential(1.5, 200)
fig, (l, r) = plt.subplots(1, 2, figsize=(9, 3.5))
l.ecdf(a, label="A")
l.ecdf(b, label="B")
l.legend()
l.set_title("ECDF")
r.ecdf(a, complementary=True, label="A")
r.ecdf(b, complementary=True, label="B")
r.set_yscale("log")
r.set_title("survival")
print(len(l.lines), round(float(np.median(a)), 4))
fig.savefig("ecdf.png")
