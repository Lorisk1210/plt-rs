# Opening many figures in a loop without closing triggers the max_open_warning; users then close.
import warnings
import matplotlib.pyplot as plt

plt.rcParams["figure.max_open_warning"] = 5
with warnings.catch_warnings(record=True) as w:
    warnings.simplefilter("always")
    for k in range(7):
        plt.figure()
        plt.plot([0, k])
print(len(plt.get_fignums()), sum("More than 5 figures" in str(x.message) for x in w))
for num in plt.get_fignums()[:-1]:
    plt.close(num)
print(plt.get_fignums())
plt.savefig("last.png")
