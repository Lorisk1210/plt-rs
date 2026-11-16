# Old-style dashboard layout with subplot2grid: a wide top panel and three small panels below.
import numpy as np
import matplotlib.pyplot as plt

np.random.seed(42)
days = np.arange(90)
visits = 200 + 30 * np.sin(days / 7 * 2 * np.pi) + np.random.normal(0, 10, days.size)

fig = plt.figure(figsize=(9, 6))
top = plt.subplot2grid((3, 3), (0, 0), colspan=3)
top.plot(days, visits)
top.set_title("Daily visits")
a = plt.subplot2grid((3, 3), (1, 0), rowspan=2)
a.hist(visits, bins=20, orientation="horizontal")
b = plt.subplot2grid((3, 3), (1, 1), rowspan=2)
b.boxplot([visits[:30], visits[30:60], visits[60:]], tick_labels=["M1", "M2", "M3"])
c = plt.subplot2grid((3, 3), (1, 2), rowspan=2)
c.scatter(visits[:-1], visits[1:], s=5)
c.set_xlabel("today")
c.set_ylabel("tomorrow")
plt.tight_layout()
plt.savefig("visits_dashboard.png")
