# Old pylab-style script with "from pylab import *" (still seen in teaching material).
from pylab import *

x = linspace(0, 2 * pi, 100)
figure(figsize=(5, 3))
plot(x, sin(x), "r-", label="sin")
plot(x, cos(x), "b--", label="cos")
xlim(0, 2 * pi)
legend()
grid(True)
title("pylab")
savefig("pylab.png")
print(type(gca()).__name__, len(gca().lines))
