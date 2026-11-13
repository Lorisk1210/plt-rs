# The type stub of matplotlib.style: matplotlib 3.11.2's, under the
# Matplotlib License, importing pltrs' modules for matplotlib's
# (python/tools/matplotlib_stubs.py).

from collections.abc import Generator
import contextlib

from pltrs import RcParams
from pltrs.typing import RcStyleType

USER_LIBRARY_PATHS: list[str] = ...

def use(style: RcStyleType) -> None: ...
@contextlib.contextmanager
def context(
    style: RcStyleType, after_reset: bool = ...
) -> Generator[None, None, None]: ...

library: dict[str, RcParams]
available: list[str]

def reload_library() -> None: ...

__all__ = ['use', 'context', 'available', 'library', 'reload_library']
