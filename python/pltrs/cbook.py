"""
A collection of utility functions and classes (matplotlib's ``cbook``).

These are inherently Python: they work on Python objects (keyword
dictionaries, sequences) rather than on what plt-rs draws.
"""

import contextlib
import time
from pathlib import Path

from . import _api


def normalize_kwargs(kw, alias_mapping=None):
    """
    Helper function to normalize kwarg inputs.

    Parameters
    ----------
    kw : dict or None
        A dict of keyword arguments.  None is explicitly supported and treated
        as an empty dict, to support functions with an optional parameter of
        the form ``props=None``.

    alias_mapping : Artist subclass or Artist instance
        A mapping between a canonical name to a list of aliases, in order of
        precedence from lowest to highest.

        If an Artist subclass or instance is passed, use its properties alias
        mapping.

    Raises
    ------
    TypeError
        To match what Python raises if invalid arguments/keyword arguments are
        passed to a callable.
    """
    from .artist import Artist

    if (isinstance(alias_mapping, type) and issubclass(alias_mapping, Artist)
            or isinstance(alias_mapping, Artist)):
        alias_to_prop = getattr(alias_mapping, "_alias_to_prop", {})
    else:
        if alias_mapping is None:
            alias_mapping = {}
        _api.warn_deprecated("3.11", message=(
            "Passing a dict or None as alias_mapping to normalize_kwargs is "
            "deprecated since %(since)s and support will be removed "
            "%(removal)s; pass an Artist instance or type instead."))
        alias_to_prop = {alias: prop for prop, aliases in alias_mapping.items()
                         for alias in aliases}

    if kw is None:
        return {}

    canonicalized = {alias_to_prop.get(k, k): v for k, v in kw.items()}
    if len(canonicalized) == len(kw):
        return canonicalized

    canonical_to_seen = {}
    for k in kw:
        canonical = alias_to_prop.get(k, k)
        if canonical in canonical_to_seen:
            raise TypeError(f"Got both {canonical_to_seen[canonical]!r} and "
                            f"{k!r}, which are aliases of one another")
        canonical_to_seen[canonical] = k


@contextlib.contextmanager
def _lock_path(path):
    """
    Context manager for locking a path.

    Usage::

        with _lock_path(path):
            ...

    Another thread or process that attempts to lock the same path will wait
    until this context manager is exited.

    The lock is implemented by creating a temporary file in the parent
    directory, so that directory must exist and be writable.
    """
    path = Path(path)
    lock_path = path.with_name(path.name + ".matplotlib-lock")
    retries = 50
    sleeptime = 0.1
    for _ in range(retries):
        try:
            with lock_path.open("xb"):
                break
        except FileExistsError:
            time.sleep(sleeptime)
    else:
        raise TimeoutError("""\
Lock error: Matplotlib failed to acquire the following lock file:
    {}
This maybe due to another process holding this lock file.  If you are sure no
other Matplotlib process is running, remove this file and try again.""".format(
            lock_path))
    try:
        yield
    finally:
        lock_path.unlink()
