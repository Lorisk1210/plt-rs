# plt.subplot called twice with the same spec returns the same axes; with other kwargs it replaces it.
import matplotlib.pyplot as plt

plt.figure(figsize=(6, 3))
a1 = plt.subplot(1, 2, 1)
a2 = plt.subplot(1, 2, 1)
print(a1 is a2)
plt.plot([1, 2, 3])
b1 = plt.subplot(1, 2, 2)
b2 = plt.subplot(1, 2, 2, projection="polar")
print(b1 is b2, len(plt.gcf().axes), b2.name)
plt.plot([0, 1, 2], [1, 2, 1])
plt.savefig("subplot_twice.png")
