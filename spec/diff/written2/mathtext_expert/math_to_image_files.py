# mathtext.math_to_image into PNG, SVG and PDF buffers and files, with dpi and color; sizes of what it writes.
import io
import matplotlib
matplotlib.use("agg")
from matplotlib import mathtext
from matplotlib.font_manager import FontProperties

exprs = [r"$E=mc^2$", r"$\nabla\cdot\vec{E}=\frac{\rho}{\epsilon_0}$", r"$\left(\frac{a}{b}\right)^{n}$"]
for e in exprs:
    for fmt in ("png", "svg", "pdf"):
        buf = io.BytesIO()
        depth = mathtext.math_to_image(e, buf, dpi=100, format=fmt, prop=FontProperties(size=14))
        data = buf.getvalue()
        print(fmt, round(depth, 3), data[:4], len(data) > 200)
d = mathtext.math_to_image(r"$\oint_C \mathbf{F}\cdot d\mathbf{r}$", "line_integral.png", dpi=150, color="navy")
print(round(d, 3))
d2 = mathtext.math_to_image(r"$x$", "x_small.png", dpi=50)
d3 = mathtext.math_to_image(r"$x$", "x_large.png", dpi=200)
print(round(d2, 3), round(d3, 3))
from PIL import Image
for name in ("line_integral.png", "x_small.png", "x_large.png"):
    with Image.open(name) as im:
        print(name, im.size, im.mode)
try:
    mathtext.math_to_image(r"$\frac{1}$", io.BytesIO())
except ValueError as err:
    print("ValueError:", str(err).splitlines()[0])
svg = io.BytesIO()
mathtext.math_to_image(r"$\alpha$", svg, format="svg")
print(svg.getvalue().count(b"<path") + svg.getvalue().count(b"<use") > 0)
