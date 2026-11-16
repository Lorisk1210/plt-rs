# Link the x axes of two separately created axes after the fact with sharex(), then zoom one.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(6, 5))
top = fig.add_subplot(2, 1, 1)
bottom = fig.add_subplot(2, 1, 2)
x = np.linspace(0, 20, 400)
top.plot(x, np.sin(x))
bottom.plot(x, np.cumsum(np.sin(x)) * 0.05)
bottom.sharex(top)
top.set_xlim(5, 10)
print(bottom.get_xlim())
top.tick_params(labelbottom=False)
bottom.set_xlabel("x")
fig.savefig("linked.png")
