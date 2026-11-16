# Reuse figures by number and by label across a loop: plt.figure(1) again draws into the same figure.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 1, 20)
for k in range(3):
    plt.figure(1)
    plt.plot(x, x ** (k + 1), label=f"p{k + 1}")
    plt.figure("residuals")
    plt.plot(x, x ** (k + 1) - x, label=f"r{k + 1}")
plt.figure(1)
plt.legend()
plt.savefig("powers.png")
plt.figure("residuals")
plt.legend()
plt.savefig("residuals.png")
print(plt.get_fignums(), plt.get_figlabels())
print(len(plt.figure(1).axes[0].lines), plt.fignum_exists("residuals"))
