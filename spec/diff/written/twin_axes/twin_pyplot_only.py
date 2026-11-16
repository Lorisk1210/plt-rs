# The pyplot-only twinx idiom with plt.twinx() and plt.sca juggling, as in older tutorials.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(1, 13)
plt.figure(figsize=(6, 3.5))
plt.bar(x, x ** 2, color="lightgray", label="squares")
plt.ylabel("squares")
ax2 = plt.twinx()
plt.plot(x, np.sqrt(x), "k-o", label="roots")
plt.ylabel("roots")
print(plt.gca() is ax2)
plt.title("Squares and roots")
plt.xlim(0, 13)
plt.legend(loc="upper center")
plt.savefig("pyplot_twin.png")
print(len(plt.gcf().axes))
