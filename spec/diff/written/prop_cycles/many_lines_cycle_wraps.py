# 15 lines wrap around the 10-colour default cycle; users then switch to a colormap instead.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 1, 20)
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
for k in range(15):
    a.plot(x, x * k)
a.set_title("default cycle wraps")
b.set_prop_cycle(color=plt.cm.tab20(np.linspace(0, 1, 15)))
for k in range(15):
    b.plot(x, x * k)
b.set_title("tab20 sampled")
print(a.lines[0].get_color() == a.lines[10].get_color(), b.lines[0].get_color())
fig.savefig("wrap.png")
