"""Cells run in an IPython shell, as Jupyter runs them, for the corpora
whose manifest table asks for it (``shell = "ipython"``).

A notebook may need the shell Jupyter gives it: ``get_ipython()``, a magic
it runs (``%timeit``, ``%%capture``), one a module it imports registers
(DataQnA's ``utils`` registers a cell magic as it is imported). `python_of`
makes each cell of such a program Python as IPython's input transformer
makes it (a magic a ``get_ipython()`` call, a shell escape a ``system``
call), dropping a cell that is still no Python, and `start` makes the
shell whose namespace is the program's. The magics choosing matplotlib's
backend never reach it (``corpora.shell_cell``): the tester shows and
closes each cell's figures as the inline backend does already, under
either implementation, so the shell changes neither which figures there
are nor their size. Both implementations run in the same shell.

IPython is imported where the shell is used, in the corpora's venvs.
"""

from __future__ import annotations

import ast
import re
from typing import Any

#: The line starting a cell of a script in the percent format, with its
#: number (``corpora.CELL_MARK``).
_CELL = re.compile(r"^(# %% \[\d+\]\n)", re.M)


def python_of(script: str) -> str:
    """The cells of *script* (the percent format) as IPython's input
    transformer makes them Python; a cell it cannot make Python left out,
    as a notebook's cell of no Python is left out of the others."""
    from IPython.core.inputtransformer2 import TransformerManager

    manager = TransformerManager()
    parts = _CELL.split(script)
    kept = [parts[0]]
    for marker, cell in zip(parts[1::2], parts[2::2]):
        try:
            code = manager.transform_cell(cell)
            ast.parse(code)
        except (SyntaxError, ValueError):
            continue
        kept += [marker, code if code.endswith("\n") else code + "\n"]
    return "".join(kept)


def start(namespace: dict[str, Any]) -> Any:
    """The IPython shell the program runs in, *namespace* its user
    namespace, ``get_ipython`` in it as Jupyter's kernel puts it in the
    builtins."""
    from IPython.core.interactiveshell import InteractiveShell

    shell = InteractiveShell.instance(user_ns=namespace)
    namespace["get_ipython"] = shell.get_ipython
    return shell
