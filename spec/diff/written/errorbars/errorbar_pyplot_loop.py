# pyplot errorbar in a loop with offset x positions so overlapping groups stay readable.
import numpy as np
import matplotlib.pyplot as plt

np.random.seed(12)
doses = np.array([1, 2, 5, 10, 20])
plt.figure(figsize=(6, 4))
for i, strain in enumerate(["A", "B", "C"]):
    resp = 100 / (1 + (10 / doses) ** (1 + i * 0.3)) + np.random.normal(0, 3, doses.size)
    plt.errorbar(doses * (1 + 0.05 * (i - 1)), resp, yerr=5, fmt="o-", capsize=3, label=f"strain {strain}")
plt.xscale("log")
plt.xlabel("dose [mg]")
plt.ylabel("response %")
plt.legend()
plt.title("Dose response")
plt.savefig("dose.png")
print(len(plt.gca().containers))
