"""
What holds a leaking scene's objects: ``python -m leaks.holders SCENE``.

Runs the scene once to warm up, again with weak references, drops it and
collects garbage as the probe does; then, if objects survived, it looks
for the *holders*: objects that Python's collector cannot account for
every reference to (their reference count is larger than the references
the tracked objects hold), from which a survivor can be reached. Native
code holds them: under pltrs, the Python objects pltrs (Rust) keeps
(callbacks, the program's functions and classes); their kinds, and a
path from each to a survivor, tell which native registry or closure
keeps the figure. A survivor held by a tracked object outside the scene
(a module's dictionary, a cache) shows in its referrers, printed too.
"""

import gc
import sys
import types
import weakref

from . import probe
from . import scenes as catalogue

#: How many objects a path search visits at most.
SEARCH_LIMIT = 200_000


def _incoming():
    """How many references the collector's tracked objects hold to each
    object, by id."""
    counts = {}
    for holder in gc.get_objects():
        for referent in gc.get_referents(holder):
            counts[id(referent)] = counts.get(id(referent), 0) + 1
    return counts


def _path(start, targets, module_dicts):
    """A path of references from *start* to an object of *targets* (ids),
    breadth first, through no module (`_foreign`); None if none is
    found."""
    parents = {id(start): None}
    queue, objects = [start], {id(start): start}
    while queue and len(parents) < SEARCH_LIMIT:
        current = queue.pop(0)
        if id(current) in targets and current is not start:
            path = [current]
            while parents[id(path[-1])] is not None:
                path.append(objects[parents[id(path[-1])]])
            return path[::-1]
        for referent in gc.get_referents(current):
            if id(referent) not in parents and not _foreign(referent, module_dicts):
                parents[id(referent)] = id(current)
                objects[id(referent)] = referent
                queue.append(referent)
    return None


def _module_dicts():
    return {id(vars(module)) for module in list(sys.modules.values()) if module is not None}


def _foreign(obj, module_dicts=frozenset()):
    """Whether *obj* is a module, a module's dictionary, or a class or
    function no scene defined: they reach everything (through their
    module), and hold no figure of their own."""
    if isinstance(obj, (weakref.ProxyType, weakref.CallableProxyType)):
        # (A proxy holds nothing; a dead one answers nothing.)
        return True
    if isinstance(obj, types.ModuleType) or id(obj) in module_dicts:
        return True
    if isinstance(obj, (type, types.FunctionType)):
        return getattr(obj, "__module__", None) != "__leaks__"
    return False


def _key_to(container, item):
    """The key or index under which *container* holds *item*, if it is a
    dict, list or tuple holding it."""
    if isinstance(container, dict):
        for key, value in container.items():
            if value is item or key is item:
                return f"[{key!r}]"[:60]
    elif isinstance(container, (list, tuple)):
        for index, value in enumerate(container):
            if value is item:
                return f"[{index}]"
    return ""


def _print_path(path, indent="    "):
    for before, step in zip(path, path[1:]):
        print(f"{indent}-> {_key_to(before, step)} {_describe(step)}")


def _describe(obj):
    text = repr(obj) if not isinstance(obj, (dict, list, tuple)) else f"<{type(obj).__name__} len={len(obj)}>"
    return f"{type(obj).__module__}.{type(obj).__qualname__}: {text[:140]}"


def _from_module(target, module_dicts):
    """A chain of referrers from a module's dictionary down to *target*,
    breadth first; None if none is found."""
    children = {id(target): None}
    queue, objects = [target], {id(target): target}
    frames = {id(frame) for frame in gc.get_objects() if isinstance(frame, types.FrameType)}
    while queue and len(children) < SEARCH_LIMIT:
        current = queue.pop(0)
        for referrer in gc.get_referrers(current):
            if id(referrer) in children or id(referrer) in frames or referrer is queue:
                continue
            children[id(referrer)] = id(current)
            objects[id(referrer)] = referrer
            if id(referrer) in module_dicts:
                chain = [referrer]
                while children[id(chain[-1])] is not None:
                    chain.append(objects[children[id(chain[-1])]])
                return chain
            queue.append(referrer)
    return None


def holders(targets):
    """The objects holding the objects of ids *targets* that the collector
    cannot account for, each with a path to one of them."""
    incoming = _incoming()
    module_dicts = _module_dicts()
    found = []
    for candidate in gc.get_objects():
        # getrefcount's own argument and the loop's variable.
        extra = sys.getrefcount(candidate) - 2 - incoming.get(id(candidate), 0)
        if (extra <= 0 or _foreign(candidate, module_dicts)
                or isinstance(candidate, types.FrameType)):
            continue
        path = _path(candidate, targets, module_dicts)
        if path is not None:
            found.append((candidate, extra, path))
    return found


def main():
    """Prints the holders of the scene the command line names."""
    import pltrs
    pltrs.use("agg")
    name = sys.argv[1]
    scene = catalogue.all_scenes()[name]
    base = probe._base_namespace()
    if scene.kept:
        # What the third update removed, once the fourth ran.
        namespace = dict(base)
        probe._run(scene.make, namespace)
        references = []
        for count in range(1, 5):
            namespace["i"] = count
            probe._run(scene.update, namespace)
            if count == 3:
                references = [(name, weakref.ref(namespace[name])) for name in scene.removed]
        gc.collect()
    else:
        probe._once(scene, base)
        probe._close_all()
        namespace, figures = probe._once(scene, base)
        references = probe._references(figures, namespace,
                                        probe._scene_classes(namespace, base))
        probe._forget(namespace)
        del namespace, figures
        probe._close_all()
        gc.collect()
    alive = [reference for _, reference in references if reference() is not None]
    del references
    print(f"{len(alive)} objects survive")
    if not alive:
        return 0
    targets = {id(reference()) for reference in alive}
    for holder, extra, path in holders(targets):
        print(f"\nheld from outside ({extra} unseen reference(s)): {_describe(holder)}")
        _print_path(path)
        del holder, path
    figures = [reference for reference in alive
               if type(reference()).__name__.endswith("Figure")] or alive[:1]
    if figures:
        chain = _from_module(figures[0](), _module_dicts())
        if chain:
            print("\nheld by a module's name:")
            print(f"    {_describe(chain[0])}")
            _print_path(chain)
    return 0


if __name__ == "__main__":
    sys.exit(main())
