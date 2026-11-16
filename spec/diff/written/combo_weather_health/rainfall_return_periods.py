# Annual maximum rainfall: bars by year, exceedance probability plot on log axes, and a fitted Gumbel line.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(155)
years = np.arange(1980, 2024)
annual_max = rng.gumbel(60, 15, years.size)
sorted_max = np.sort(annual_max)[::-1]
rank = np.arange(1, years.size + 1)
p_exc = rank / (years.size + 1)
T = 1 / p_exc
fig, (a, b) = plt.subplots(1, 2, figsize=(11, 4))
a.bar(years, annual_max, color=np.where(annual_max > np.percentile(annual_max, 90), "C3", "C0"))
a.set_ylabel("max daily rain [mm]")
b.semilogx(T, sorted_max, "o", label="observed")
beta = annual_max.std() * np.sqrt(6) / np.pi
mu = annual_max.mean() - 0.5772 * beta
Tf = np.logspace(0.01, 2.5, 100)
b.semilogx(Tf, mu - beta * np.log(-np.log(1 - 1 / Tf)), "k", label="Gumbel fit")
b.set_xlabel("return period [years]")
b.legend()
print(round(float(mu), 3), round(float(beta), 3), round(float(sorted_max[0]), 3))
fig.savefig("return_periods.png", dpi=50)
