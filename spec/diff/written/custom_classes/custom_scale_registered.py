# Register a custom ScaleBase (a Mercator-like latitude scale) and use it by name.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib import scale as mscale
from matplotlib import transforms as mtransforms


class SquareRootScale(mscale.ScaleBase):
    name = "squareroot"

    def get_transform(self):
        return self.SqrtTransform()

    def set_default_locators_and_formatters(self, axis):
        axis.set_major_locator(plt.MaxNLocator(6))
        axis.set_major_formatter(plt.ScalarFormatter())

    class SqrtTransform(mtransforms.Transform):
        input_dims = output_dims = 1

        def transform_non_affine(self, a):
            return np.sqrt(np.clip(a, 0, None))

        def inverted(self):
            return SquareRootScale.InvertedSqrtTransform()

    class InvertedSqrtTransform(mtransforms.Transform):
        input_dims = output_dims = 1

        def transform_non_affine(self, a):
            return np.asarray(a) ** 2

        def inverted(self):
            return SquareRootScale.SqrtTransform()


mscale.register_scale(SquareRootScale)
x = np.arange(0, 100)
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot(x, x)
ax.set_yscale("squareroot")
print(ax.get_yscale(), "squareroot" in mscale.get_scale_names())
fig.savefig("sqrt_scale.png")
