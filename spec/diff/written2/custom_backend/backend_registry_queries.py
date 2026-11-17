# backend_registry queries: list backends by filter, resolve names and gui frameworks, is_valid_backend, and module:// names.
import matplotlib
matplotlib.use("agg")
from matplotlib.backends import backend_registry, BackendFilter

inter = backend_registry.list_builtin(BackendFilter.INTERACTIVE)
non = backend_registry.list_builtin(BackendFilter.NON_INTERACTIVE)
print(sorted(non))
print(len(inter) > 5, "qtagg" in inter, "agg" in non)
for name in ("agg", "AGG", "QtAgg", "pdf", "module://some.backend", "nonexistent", "inline", "tkagg"):
    print(name, backend_registry.is_valid_backend(name))
for name in ("agg", "qtagg", "tkagg", "gtk4agg", "pdf", "macosx", "webagg"):
    try:
        print(name, backend_registry.resolve_backend(name))
    except Exception as e:
        print(name, type(e).__name__, str(e)[:50])
for gui in ("qt", "tk", "gtk3", "headless", "wx"):
    try:
        print(gui, backend_registry.resolve_gui_or_backend(gui))
    except Exception as e:
        print(gui, type(e).__name__, str(e)[:50])
print(backend_registry.backend_for_gui_framework("qt"), backend_registry.backend_for_gui_framework("headless"))
print(sorted(backend_registry.list_gui_frameworks())[:5])
print(len(backend_registry.list_all()) >= len(inter) + len(non))
print(matplotlib.rcsetup.validate_backend("Agg"), matplotlib.rcsetup.validate_backend("module://x.y"))
try:
    matplotlib.rcsetup.validate_backend("bogus")
except ValueError as e:
    print("ValueError", str(e)[:60])
print(matplotlib.get_backend(), matplotlib.rcParams._get_backend_or_none())
