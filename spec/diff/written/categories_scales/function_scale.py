# A square-root custom axis scale using set_yscale('function') for count data.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(0, 20)
counts = (x ** 2.2).round()
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
a.bar(x, counts)
a.set_title("linear")
b.bar(x, counts)
b.set_yscale("function", functions=(np.sqrt, np.square))
b.set_title("sqrt scale")
b.set_ylim(0, counts.max())
print(b.get_yscale())
fig.tight_layout()
fig.savefig("sqrt_scale.png")
