# Scripts toggle plt.ion()/plt.ioff(), call plt.draw() and plt.pause(); with Agg these do nothing visible.
import warnings
import matplotlib.pyplot as plt

print(plt.isinteractive())
plt.ion()
print(plt.isinteractive())
fig, ax = plt.subplots(figsize=(4, 3))
line, = ax.plot([0, 1], [0, 1])
for k in range(3):
    line.set_ydata([0, k + 1])
    ax.set_ylim(0, 4)
    plt.draw()
    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        plt.pause(0.001)
plt.ioff()
print(plt.isinteractive(), list(line.get_ydata()))
plt.savefig("interactive.png")
