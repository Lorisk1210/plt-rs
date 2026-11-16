# Respondent demographics: an age pyramid, a pie of education, and a bar of region with matching palette.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(77)
ages = np.arange(18, 80, 5)
men = rng.integers(10, 60, ages.size); women = rng.integers(10, 60, ages.size)
fig = plt.figure(figsize=(12, 4.5))
a = fig.add_subplot(1, 3, 1)
a.barh(ages, -men, height=4, color="steelblue", label="men")
a.barh(ages, women, height=4, color="lightcoral", label="women")
a.set_xticks(a.get_xticks(), [str(abs(int(t))) for t in a.get_xticks()])
a.legend()
b = fig.add_subplot(1, 3, 2)
edu = {"school": 30, "bachelor": 45, "master": 20, "phd": 5}
wedges, *_ = b.pie(edu.values(), labels=edu.keys(), autopct="%d%%", colors=plt.cm.Set3.colors)
c = fig.add_subplot(1, 3, 3)
c.bar(["north", "south", "east", "west"], [120, 90, 60, 80], color=[w.get_facecolor() for w in wedges])
fig.tight_layout()
print(int(men.sum() + women.sum()), [t.get_text() for t in a.get_xticklabels()][:3])
fig.savefig("demographics.png", dpi=50)
