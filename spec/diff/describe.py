"""What a value is, as the differential tester compares it.

``Describer.describe(value)`` turns any value a program computed into JSON
the comparison reads: plain values (numbers, strings, containers, numpy
arrays and scalars) by type and content; matplotlib's value classes
(``Path``, ``Bbox``, affine transforms) by type and numbers; every other
object by its type, its public classes along the MRO and, when it was
described before, the index of that first description, so that identity
(``ax.bbox is ax.bbox``) is compared too.

Describing never changes the program's state: no getter of an artist is
called. Names are matplotlib's under both implementations: the run under
pltrs gives a ``Names`` that renames ``pltrs.x`` to ``matplotlib.x``.
"""

from __future__ import annotations

import math
import os
import re
import sys
import tempfile
import types
from typing import Any

#: Elements of an array, or items of a container, kept in full; larger ones
#: are summarised.
MAX_ITEMS = 2000
#: How deep containers are described.
MAX_DEPTH = 6

#: Memory addresses, and what is left of one a program blanked with a
#: lowercase pattern: Windows writes them in capitals after leading zeros
#: (``0x000001BA38FFD7A0``), so ``0x[0-9a-f]+`` leaves ``0x?BA38FFD7A0``.
_ADDRESS = re.compile(r"0x(?:\?[0-9A-F]{4,}|[0-9a-fA-F]{4,})")
#: The random names ``tempfile`` gives temporary files and directories.
_TEMPORARY = re.compile(re.escape(tempfile.gettempdir() + os.sep) + r"tmp[\w-]{8}")


class Names:
    """How the implementation under test spells matplotlib's names.

    ``prefixes`` maps the implementation's module prefixes to matplotlib's
    (empty for matplotlib itself); ``package_dirs`` are the directories of
    the packages, which messages naming files replace with ``<package>``,
    ``config_dir`` the run's configuration directory, ``work_dir`` its
    working directory and ``venv_dir`` its venv (each implementation has
    its own; absolute or relative to the working directory), replaced
    with ``<config>``, ``<cwd>`` and ``<venv>``, and ``run_dir`` the
    directory the run writes to (named after the implementation: `mpl`
    or `pltrs`), where a corpus' tree is laid out, with ``<run>``. The
    random names of temporary files become ``<tmp>``.
    """

    def __init__(self, prefixes: list[tuple[str, str]], package_dirs: list[str],
                 config_dir: str | None = None, work_dir: str | None = None,
                 venv_dir: str | None = None, run_dir: str | None = None) -> None:
        self.prefixes = prefixes
        self.package_dirs = sorted(package_dirs, key=len, reverse=True)
        self.config_dir = config_dir
        self.work_dir = work_dir
        self.venv_dir = venv_dir
        self.run_dir = run_dir
        # (The venv as a path relative to the working directory, as numba
        # names the files of its errors.)
        self.venv_relative = os.path.relpath(venv_dir) if venv_dir else None
        self._word = [
            (re.compile(r"(?<![\w.])" + re.escape(alias) + r"(?![\w])"), real)
            for alias, real in prefixes
        ]

    def module(self, name: str) -> str:
        """matplotlib's name of the module ``name``."""
        for alias, real in self.prefixes:
            if name == alias or name.startswith(alias + "."):
                return real + name[len(alias):]
        return name

    def text(self, text: str) -> str:
        """``text`` with the implementation's names and paths made
        matplotlib's, and memory addresses blanked."""
        for directory in self.package_dirs:
            text = _replace_path(text, directory, "<package>")
        if self.config_dir:
            text = _replace_path(text, self.config_dir, "<config>")
        if self.work_dir:
            text = _replace_path(text, self.work_dir, "<cwd>")
        if self.run_dir:
            text = _replace_path(text, self.run_dir, "<run>")
        if self.venv_dir:
            text = _replace_path(text, self.venv_dir, "<venv>")
        if self.venv_relative:
            text = _replace_path(text, self.venv_relative, "<venv>")
        for pattern, real in self._word:
            text = pattern.sub(real, text)
        return _ADDRESS.sub("0x?", _TEMPORARY.sub("<tmp>", text))

    def class_name(self, cls: type) -> str:
        """The class' module and qualified name, in matplotlib's names."""
        return f"{self.module(getattr(cls, '__module__', '?') or '?')}.{cls.__qualname__}"


def _replace_path(text: str, path: str, placeholder: str) -> str:
    """``text`` with ``path`` replaced by ``placeholder``, as written and
    as a ``repr`` writes it (Windows' backslashes doubled)."""
    return text.replace(path.replace("\\", "\\\\"), placeholder).replace(path, placeholder)


def public_mro(cls: type) -> list[str]:
    """The qualified names of the public classes along ``cls``' MRO, but
    ``object``: what ``isinstance`` checks a program makes can see."""
    return [
        c.__qualname__
        for c in cls.__mro__[1:]
        if c is not object and not c.__qualname__.startswith("_")
    ]


def _float(value: float) -> Any:
    if math.isnan(value):
        return "nan"
    if math.isinf(value):
        return "inf" if value > 0 else "-inf"
    return value


class Describer:
    """Describes the values of one run, remembering the objects described
    (and keeping them alive, so that their ids stay theirs)."""

    def __init__(self, names: Names) -> None:
        self.names = names
        self._seen: dict[int, int] = {}
        self._kept: list[Any] = []
        self._np = sys.modules.get("numpy")

    def describe(self, value: Any, depth: int = 0) -> dict[str, Any]:
        """The JSON description of ``value``."""
        try:
            return self._describe(value, depth)
        except Exception as err:  # noqa: BLE001 - describing must not fail the program
            return {"k": "undescribable", "type": self.names.class_name(type(value)),
                    "error": self.names.text(f"{type(err).__name__}: {err}")}

    def _describe(self, value: Any, depth: int) -> dict[str, Any]:
        np = self._np
        if value is None:
            return {"k": "none"}
        if np is not None and isinstance(value, np.ndarray):
            return self._array(value)
        if np is not None and isinstance(value, np.generic):
            return self._scalar(value)
        if isinstance(value, bool):
            return {"k": "bool", "v": value}
        if isinstance(value, int) and type(value) is int:
            return {"k": "int", "v": value}
        if isinstance(value, float) and type(value) is float:
            return {"k": "float", "v": _float(value)}
        if isinstance(value, complex) and type(value) is complex:
            return {"k": "complex", "v": [_float(value.real), _float(value.imag)]}
        if isinstance(value, str) and type(value) is str:
            return {"k": "str", "v": self.names.text(value)}
        if isinstance(value, (bytes, bytearray)):
            return {"k": type(value).__name__, "len": len(value)}
        if type(value) in (tuple, list):
            return self._sequence(type(value).__name__, value, depth)
        if type(value) in (set, frozenset):
            items = sorted(value, key=repr)
            return self._sequence(type(value).__name__, items, depth)
        if type(value) is dict:
            return self._dict(value, depth)
        if isinstance(value, type):
            return {"k": "class", "name": self.names.class_name(value)}
        if isinstance(value, types.ModuleType):
            return {"k": "module", "name": self.names.module(value.__name__)}
        if isinstance(value, (types.FunctionType, types.BuiltinFunctionType,
                              types.MethodType, types.BuiltinMethodType)):
            return {"k": "callable", "name": getattr(value, "__qualname__", "?")}
        return self._object(value, depth)

    def _array(self, value: Any) -> dict[str, Any]:
        np = self._np
        description: dict[str, Any] = {
            "k": "ndarray",
            "type": self.names.class_name(type(value)),
            "dtype": str(value.dtype),
            "shape": list(value.shape),
        }
        if isinstance(value, np.ma.MaskedArray):
            mask = np.ma.getmaskarray(value)
            description["mask"] = self._flat(mask)
            value = np.ma.getdata(value)
        description["v"] = self._flat(value)
        return description

    def _flat(self, value: Any) -> Any:
        """The elements of an array, or a summary of a large one."""
        np = self._np
        flat = value.ravel()
        if value.dtype.kind in "biufc":
            if flat.size > MAX_ITEMS:
                finite = flat[np.isfinite(flat)] if value.dtype.kind in "fc" else flat
                # (A sum overflowing is the summary's, no warning of the
                # program's.)
                with np.errstate(all="ignore"):
                    total = finite.sum() if finite.size else None
                return {
                    "size": int(flat.size),
                    "head": [self._element(x) for x in flat[:50]],
                    "sum": None if total is None else self._element(total),
                    "min": self._element(finite.min()) if finite.size else None,
                    "max": self._element(finite.max()) if finite.size else None,
                    "nonfinite": int(flat.size - finite.size),
                }
            return [self._element(x) for x in flat]
        if flat.size > MAX_ITEMS:
            return {"size": int(flat.size), "head": [self.describe(x, MAX_DEPTH) for x in flat[:50]]}
        return [self.describe(x, MAX_DEPTH) for x in flat]

    def _element(self, x: Any) -> Any:
        if isinstance(x, (bool, self._np.bool_)):
            return bool(x)
        if isinstance(x, (int, self._np.integer)):
            return int(x)
        if isinstance(x, (complex, self._np.complexfloating)):
            return [_float(float(x.real)), _float(float(x.imag))]
        return _float(float(x))

    def _scalar(self, value: Any) -> dict[str, Any]:
        description = {"k": "npscalar", "dtype": str(value.dtype)}
        if value.dtype.kind in "biufc":
            description["v"] = self._element(value)
        else:
            description["v"] = self.names.text(str(value))
        return description

    def _sequence(self, kind: str, items: Any, depth: int) -> dict[str, Any]:
        items = list(items)
        if depth >= MAX_DEPTH:
            return {"k": kind, "len": len(items)}
        kept = items[:MAX_ITEMS // 10]
        return {"k": kind, "len": len(items), "items": [self.describe(x, depth + 1) for x in kept]}

    def _dict(self, value: dict, depth: int) -> dict[str, Any]:
        if depth >= MAX_DEPTH:
            return {"k": "dict", "len": len(value)}
        items = list(value.items())[:MAX_ITEMS // 10]
        return {
            "k": "dict",
            "len": len(value),
            "items": [[self.describe(k, depth + 1), self.describe(v, depth + 1)] for k, v in items],
        }

    def _object(self, value: Any, depth: int) -> dict[str, Any]:
        cls = type(value)
        description: dict[str, Any] = {
            "k": "obj",
            "type": self.names.class_name(cls),
            "mro": public_mro(cls),
        }
        key = id(value)
        if key in self._seen:
            description["same_as"] = self._seen[key]
        else:
            self._seen[key] = len(self._seen)
            self._kept.append(value)
            description["first"] = self._seen[key]
        description.update(self._value_class(value, cls, depth))
        return description

    def _value_class(self, value: Any, cls: type, depth: int) -> dict[str, Any]:
        """The numbers of matplotlib's value classes, read without side
        effects: a path's vertices and codes, a box's corners, the matrix
        of an affine transform."""
        names = {c.__qualname__ for c in cls.__mro__}
        extra: dict[str, Any] = {}
        if "Path" in names and hasattr(value, "vertices"):
            extra["vertices"] = self.describe(value.vertices, depth + 1)
            extra["codes"] = self.describe(value.codes, depth + 1)
        elif "BboxBase" in names:
            extra["points"] = self.describe(value.get_points(), depth + 1)
        elif "Transform" in names and getattr(value, "is_affine", False):
            extra["matrix"] = self.describe(value.get_matrix(), depth + 1)
        return extra
