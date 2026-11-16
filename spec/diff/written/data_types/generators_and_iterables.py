# Data given as generators, tuples, dict views and map objects (users do this) - what works.
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 3, figsize=(10, 3))
data = {"a": 3, "b": 7, "c": 5}
axs[0].bar(data.keys(), data.values())
axs[0].set_title("dict views")
axs[1].plot(tuple(range(5)), tuple(v * v for v in range(5)))
axs[1].set_title("tuples")
try:
    axs[2].plot((v for v in range(5)), (v * 2 for v in range(5)))
    print("generators accepted")
except Exception as err:
    print(type(err).__name__, str(err)[:80])
axs[2].plot(list(map(float, "1 2 3".split())), [3, 1, 2])
axs[2].set_title("map -> list")
fig.savefig("iterables.png")
