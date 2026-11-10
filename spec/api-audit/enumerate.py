"""Enumerate matplotlib's public API by introspection, as JSON lines.

    python enumerate.py [--nested]

Each record: module, owner (class name or ""), name, kind
(function/class/method/property/classmethod/staticmethod/attribute/
instance_attribute/kwarg), deprecated flag, and for classes their bases.
With ``--nested``, the members of the classes nested in classes too
(``LambertAxes.InvertedLambertTransform.inverted``), owned by the nested
class' qualified name.
"""
import importlib
import inspect
import json
import sys
import warnings

import matplotlib

matplotlib.use("agg")
warnings.simplefilter("ignore")

MODULES = """
matplotlib matplotlib.pyplot matplotlib.figure matplotlib.axes matplotlib.axis
matplotlib.artist matplotlib.lines matplotlib.patches matplotlib.collections
matplotlib.text matplotlib.image matplotlib.contour matplotlib.colors
matplotlib.colorbar matplotlib.cm matplotlib.ticker matplotlib.dates
matplotlib.scale matplotlib.transforms matplotlib.path matplotlib.markers
matplotlib.legend matplotlib.legend_handler matplotlib.offsetbox
matplotlib.gridspec matplotlib.layout_engine matplotlib.widgets
matplotlib.animation matplotlib.backend_bases matplotlib.backend_tools
matplotlib.font_manager matplotlib.mathtext matplotlib.patheffects
matplotlib.quiver matplotlib.streamplot matplotlib.sankey matplotlib.table
matplotlib.tri matplotlib.units matplotlib.style matplotlib.rcsetup
matplotlib.cbook matplotlib.projections matplotlib.projections.polar
matplotlib.projections.geo matplotlib.spines matplotlib.container
matplotlib.hatch matplotlib.bezier matplotlib.textpath matplotlib.typing
matplotlib.colorizer matplotlib.category matplotlib.mlab
mpl_toolkits.mplot3d mpl_toolkits.mplot3d.axes3d mpl_toolkits.mplot3d.art3d
mpl_toolkits.mplot3d.axis3d mpl_toolkits.mplot3d.proj3d
mpl_toolkits.axes_grid1 mpl_toolkits.axes_grid1.axes_divider
mpl_toolkits.axes_grid1.axes_grid mpl_toolkits.axes_grid1.axes_rgb
mpl_toolkits.axes_grid1.axes_size mpl_toolkits.axes_grid1.anchored_artists
mpl_toolkits.axes_grid1.inset_locator mpl_toolkits.axes_grid1.mpl_axes
mpl_toolkits.axes_grid1.parasite_axes
mpl_toolkits.axisartist mpl_toolkits.axisartist.axislines
mpl_toolkits.axisartist.axis_artist mpl_toolkits.axisartist.grid_finder
mpl_toolkits.axisartist.grid_helper_curvelinear
mpl_toolkits.axisartist.floating_axes mpl_toolkits.axisartist.angle_helper
mpl_toolkits.axisartist.axisline_style mpl_toolkits.axisartist.parasite_axes
matplotlib.backend_managers matplotlib.inset matplotlib.stackplot
matplotlib.texmanager matplotlib.dviread matplotlib.ft2font
matplotlib.backends.registry matplotlib.backends.backend_agg
matplotlib.backends.backend_pdf matplotlib.backends.backend_svg
matplotlib.backends.backend_ps matplotlib.backends.backend_pgf
matplotlib.backends.backend_mixed matplotlib.testing
matplotlib.testing.compare matplotlib.testing.decorators
""".split()

# Packages whose names live in sub-modules: a name counts for the package
# when one of its own sub-modules defines it.
PACKAGES = {"matplotlib.axes", "matplotlib.tri", "matplotlib.projections",
            "mpl_toolkits.mplot3d", "mpl_toolkits.axes_grid1",
            "mpl_toolkits.axisartist", "matplotlib.style"}

SKIP_MEMBERS = {
    "__init__", "__repr__", "__str__", "__getstate__", "__setstate__",
    "__reduce__", "__init_subclass__", "__class_getitem__", "__hash__",
    "__eq__", "__ne__", "__weakref__", "__dict__", "__module__", "__doc__",
    "__annotations__", "__qualname__", "__slots__", "__signature__",
    "__copy__", "__deepcopy__", "__new__", "__del__", "__format__",
    "__sizeof__", "__subclasshook__", "__firstlineno__", "__static_attributes__",
    "__annotate_func__", "__annotations_cache__", "__parameters__",
    "__orig_bases__", "__abstractmethods__", "_abc_impl", "__dir__",
}

# Classes whose methods' parameters are listed (the main plotting methods).
KWARG_OWNERS = {("matplotlib.axes", "Axes"), ("mpl_toolkits.mplot3d", "Axes3D")}

# Whether the members of nested classes are listed too.
NESTED = "--nested" in sys.argv[1:]


def deprecated(obj):
    doc = (inspect.getdoc(obj) or "").strip()
    return doc.startswith("[*Deprecated*]") or doc.startswith(".. deprecated::")


def is_public(name):
    if name in SKIP_MEMBERS:
        return False
    return not name.startswith("_") or (name.startswith("__") and name.endswith("__"))


def module_of(obj):
    return getattr(obj, "__module__", None) or ""


def owned(modname, obj):
    m = module_of(obj)
    if modname in PACKAGES:
        return m == modname or m.startswith(modname + ".")
    return m == modname


seen_classes = set()
# Private base classes of public classes, with the first public user: their
# members are public API of every subclass.
private_bases = {}
records = []


def emit(**rec):
    records.append(rec)


def params_of(module, owner, name, func):
    try:
        sig = inspect.signature(func)
    except (TypeError, ValueError):
        return
    for p in sig.parameters.values():
        if p.name in ("self", "cls"):
            continue
        kind = {p.VAR_POSITIONAL: "*args", p.VAR_KEYWORD: "**kwargs"}.get(p.kind, "param")
        emit(module=module, owner=owner, name=f"{name}({p.name})",
             kind="kwarg", param_kind=kind, deprecated=False)


def class_members(module, cls, owner=None):
    owner = owner or cls.__qualname__
    for name, raw in vars(cls).items():
        if not is_public(name):
            continue
        if isinstance(raw, staticmethod):
            kind, obj = "staticmethod", raw.__func__
        elif isinstance(raw, classmethod):
            kind, obj = "classmethod", raw.__func__
        elif isinstance(raw, property):
            kind, obj = "property", raw
        elif inspect.isfunction(raw):
            kind, obj = "method", raw
        elif inspect.isclass(raw):
            kind, obj = "class", raw
        elif callable(raw):
            kind, obj = "method", raw
        else:
            kind, obj = "attribute", raw
        emit(module=module, owner=owner, name=name, kind=kind,
             deprecated=deprecated(obj) if kind != "attribute" else False)
        if kind in ("method", "classmethod", "staticmethod") and (module, owner) in KWARG_OWNERS:
            params_of(module, owner, name, obj)
        if NESTED and kind == "class" and obj.__qualname__ == f"{cls.__qualname__}.{name}":
            # A class defined in the class' body, not one it refers to.
            class_members(module, obj, f"{owner}.{name}")


def bases_of(cls):
    return [f"{b.__module__}.{b.__qualname__}" for b in cls.__bases__]


def walk(modname):
    mod = importlib.import_module(modname)
    for name in sorted(vars(mod)):
        if name.startswith("_"):
            continue
        obj = getattr(mod, name)
        if inspect.ismodule(obj):
            continue
        if not owned(modname, obj):
            continue
        if inspect.isclass(obj):
            key = (module_of(obj), obj.__qualname__)
            if key in seen_classes:
                continue
            seen_classes.add(key)
            emit(module=modname, owner="", name=name, kind="class",
                 deprecated=deprecated(obj), bases=bases_of(obj))
            class_members(modname, obj)
            for base in obj.__mro__[1:]:
                if base.__name__.startswith("_") and base is not object and base.__module__ != "builtins":
                    private_bases.setdefault(base, (modname, name))
        elif callable(obj):
            emit(module=modname, owner="", name=name, kind="function",
                 deprecated=deprecated(obj))
            if modname == "matplotlib.pyplot":
                params_of(modname, "", name, obj)
        else:
            emit(module=modname, owner="", name=name, kind="attribute", deprecated=False)


for m in MODULES:
    walk(m)

for base, (modname, user) in private_bases.items():
    emit(module=modname, owner="", name=base.__qualname__, kind="private_base",
         deprecated=False, bases=[user])
    class_members(modname, base)

# Instance attributes of the central classes, found by making them.
import matplotlib.pyplot as plt  # noqa: E402

fig, ax = plt.subplots()
line, = ax.plot([0, 1], [0, 1])
txt = ax.text(0, 0, "t")
leg = ax.legend(["a"])
im = ax.imshow([[0, 1], [1, 0]])
cb = fig.colorbar(im)
instances = {
    ("matplotlib.figure", "Figure"): fig, ("matplotlib.axes", "Axes"): ax,
    ("matplotlib.lines", "Line2D"): line, ("matplotlib.text", "Text"): txt,
    ("matplotlib.legend", "Legend"): leg, ("matplotlib.image", "AxesImage"): im,
    ("matplotlib.colorbar", "Colorbar"): cb, ("matplotlib.axis", "XAxis"): ax.xaxis,
    ("matplotlib.backend_bases", "FigureCanvasBase"): fig.canvas,
}
for (module, owner), inst in instances.items():
    cls_attrs = set(dir(type(inst)))
    for name in sorted(vars(inst)):
        if name.startswith("_") or name in cls_attrs:
            continue
        emit(module=module, owner=owner, name=name, kind="instance_attribute", deprecated=False)

for r in records:
    print(json.dumps(r))
print(f"{len(records)} records", file=sys.stderr)
