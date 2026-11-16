# Fractions that sum to less than 1 give a partial pie when normalize=False; users compare.
import matplotlib.pyplot as plt

fractions = [0.2, 0.3, 0.15]
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 4))
a.pie(fractions, labels=["x", "y", "z"], normalize=True, autopct="%.0f%%")
a.set_title("normalize=True")
b.pie(fractions, labels=["x", "y", "z"], normalize=False, autopct="%.0f%%")
b.set_title("normalize=False (partial)")
print([round(p.theta2, 3) for p in b.patches])
fig.savefig("partial.png")
fig.savefig("partial_small.png", dpi=50)
fig.savefig("pie_not_sum_to_one_80dpi.png", dpi=80)
print(fig.get_size_inches(), len(fig.axes))
plt.close(fig)
