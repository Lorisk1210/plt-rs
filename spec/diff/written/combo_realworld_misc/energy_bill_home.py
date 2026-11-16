# Household energy: monthly kWh bars vs last year, cost line on a twin axis, appliance share pie and a daily load profile.
import numpy as np
import matplotlib.pyplot as plt

months = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"]
rng = np.random.default_rng(232)
kwh = 300 + 120 * np.cos(np.arange(12) / 12 * 2 * np.pi) + rng.normal(0, 15, 12)
last = kwh * rng.uniform(0.95, 1.15, 12)
fig = plt.figure(figsize=(12, 7))
a = fig.add_subplot(2, 1, 1)
x = np.arange(12)
a.bar(x - 0.2, last, 0.4, color="0.75", label="last year")
a.bar(x + 0.2, kwh, 0.4, color="C0", label="this year")
tw = a.twinx(); tw.plot(x, kwh * 0.32, "C3o-", label="cost €")
a.set_xticks(x, months); a.legend(loc="upper center", ncols=2)
b = fig.add_subplot(2, 2, 3)
b.pie([35, 20, 15, 12, 18], labels=["heating", "kitchen", "laundry", "lights", "other"], autopct="%d%%")
c = fig.add_subplot(2, 2, 4)
h = np.arange(24)
c.fill_between(h, 0.2 + 0.8 * np.exp(-((h - 19) / 2) ** 2) + 0.4 * np.exp(-((h - 7) / 1.5) ** 2), step="mid", alpha=0.5)
c.set_xticks(range(0, 24, 4))
fig.tight_layout()
print(round(float(kwh.sum()), 2), round(float((kwh * 0.32).sum()), 2))
fig.savefig("energy_home.png", dpi=50)
