# plt.show() under a non-interactive backend: users see a warning and the script continues.
import matplotlib
import matplotlib.pyplot as plt

print(matplotlib.get_backend())
plt.plot([1, 2, 3], [1, 4, 9], "o-")
plt.title("squares")
plt.savefig("squares.png")
plt.show()
plt.figure()
plt.bar([1, 2], [3, 4])
plt.show(block=False)
print(plt.get_fignums())
