# Many series: put the legend outside the axes on the right with bbox_to_anchor and save tight.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 1, 50)
fig, ax = plt.subplots(figsize=(6, 4))
for k in range(1, 9):
    ax.plot(x, x ** (k / 3), label=f"exponent {k / 3:.2f}")
ax.legend(loc="center left", bbox_to_anchor=(1.02, 0.5), borderaxespad=0, title="curves")
ax.set_xlabel("x")
ax.set_ylabel("x^p")
fig.savefig("outside_right.png", bbox_inches="tight")
leg = ax.get_legend()
print(len(leg.get_texts()), leg.get_title().get_text())
ax.legend(loc="upper left", bbox_to_anchor=(1.0, 1.0), fontsize=8, ncol=2)
fig.savefig("outside_two_columns.png", bbox_inches="tight")
