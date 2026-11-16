# Area charts: stackplot of market shares and fill_betweenx for a vertical profile.
import numpy as np
import matplotlib.pyplot as plt

years = np.arange(2015, 2025)
a = np.linspace(50, 30, 10)
b = np.linspace(30, 35, 10)
c = 100 - a - b
fig, (s, f) = plt.subplots(1, 2, figsize=(9, 3.5))
polys = s.stackplot(years, a, b, c, labels=["incumbent", "challenger", "others"], alpha=0.85)
s.legend(loc="upper left", fontsize=8)
s.set_ylim(0, 100)
s.set_title("market share %")
depth = np.linspace(0, 100, 50)
f.fill_betweenx(depth, 0, np.exp(-depth / 30) * 10, color="skyblue")
f.invert_yaxis()
f.set_title("profile")
print(len(polys))
fig.savefig("areas.png")
