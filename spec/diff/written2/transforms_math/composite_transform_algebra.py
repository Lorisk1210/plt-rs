# Transform algebra checked numerically: composition order, inverted(), frozen(), depth, contains_branch, is_affine.
import numpy as np
from matplotlib.transforms import (Affine2D, BboxTransformTo, Bbox, IdentityTransform, blended_transform_factory,
                                   CompositeGenericTransform)


def show(name, t, pts):
    out = t.transform(pts)
    back = t.inverted().transform(out)
    print(name, type(t).__name__, t.is_affine, t.depth, np.round(out, 6).tolist(), np.allclose(back, pts))


pts = np.array([[0.0, 0.0], [1.0, 2.0], [-3.0, 0.5]])
a = Affine2D().rotate_deg(30).translate(1, 2)
b = Affine2D().scale(2, 0.5)
box = BboxTransformTo(Bbox([[10, 20], [110, 70]]))
show("a+b", a + b, pts)
show("b+a", b + a, pts)
show("a+box", a + box, pts)
show("identity", IdentityTransform() + a, pts)
chain = a + b + box
print(chain.contains_branch(b), chain.contains_branch(Affine2D()), (b + box).contains_branch(box))
print(np.round(chain.get_matrix(), 6).tolist())
frozen = chain.frozen()
b.scale(3)
print(np.round(frozen.transform(pts[1:2]), 6).tolist(), np.round(chain.transform(pts[1:2]), 6).tolist())
blend = blended_transform_factory(box, IdentityTransform())
print(type(blend).__name__, np.round(blend.transform(pts), 6).tolist(), blend.is_separable)
print(isinstance(a + b, CompositeGenericTransform), type(a + b).__name__)
diff = chain - box
print(type(diff).__name__, np.round(diff.transform(pts[1:2]), 6).tolist())
print(repr(Affine2D().get_matrix().tolist()))
inv = (a + b).inverted()
print(type(inv).__name__, np.round(inv.get_matrix(), 6).tolist())
