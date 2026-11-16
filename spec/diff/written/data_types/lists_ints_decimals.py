# Plot plain Python lists of ints, Decimals and Fractions converted to floats, plus a range object.
from decimal import Decimal
from fractions import Fraction
import matplotlib.pyplot as plt

xs = range(1, 6)
prices = [Decimal("1.10"), Decimal("1.25"), Decimal("1.05"), Decimal("1.40"), Decimal("1.33")]
shares = [Fraction(1, 2), Fraction(2, 3), Fraction(3, 4), Fraction(4, 5), Fraction(5, 6)]
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3))
a.plot(xs, prices, "o-")
a.set_title("Decimal")
b.plot(list(xs), [float(f) for f in shares], "s-")
b.plot(list(xs), shares, "x", ms=10)
b.set_title("Fraction")
print(a.lines[0].get_ydata()[:2], type(b.lines[1].get_ydata()[0]).__name__)
fig.savefig("numbers.png")
