# Sine and cosine one above the other with plt.subplot and a common title.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 2 * np.pi, 100)
plt.figure(figsize=(6, 5))
plt.subplot(2, 1, 1)
plt.plot(x, np.sin(x), "r")
plt.title("sin")
plt.subplot(2, 1, 2)
plt.plot(x, np.cos(x), "b")
plt.title("cos")
plt.suptitle("Trig functions")
plt.tight_layout()
plt.savefig("trig.png")
print(len(plt.gcf().axes))
