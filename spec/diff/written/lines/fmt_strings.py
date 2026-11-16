# Old-school format strings ('r--', 'go', 'b^:') and several lines in one plot() call.
import numpy as np
import matplotlib.pyplot as plt

t = np.arange(0.0, 5.0, 0.2)
plt.figure(figsize=(6, 4))
lines = plt.plot(t, t, "r--", t, t ** 2, "bs", t, t ** 3, "g^:")
plt.plot(t, 20 * np.ones_like(t), "k-", linewidth=0.5)
plt.plot(t, t ** 1.5, "C4o-", ms=3)
plt.axis([0, 5, 0, 60])
print(len(lines), [(l.get_color(), l.get_marker(), l.get_linestyle()) for l in lines])
plt.savefig("formats.png")
plt.xlabel("t")
plt.ylabel("t to the power k")
plt.title("Format strings")
plt.savefig("formats_titled.png", dpi=60)
