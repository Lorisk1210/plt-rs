"""
Warnings that pltrs' own code ignores or records without telling Python
that the filters changed.

`warnings.catch_warnings` marks the filters changed when it is entered and
when it is left. That empties every module's record of the warnings it has
shown (``__warningregistry__``), so under the default filters a program's
warning, shown once at a line, is shown again after each such block.

pltrs runs code of its own where matplotlib runs none: it catches up with
the ticks pltrs (Rust) computed, and it holds the warnings a program's
artist gives while it is recorded. Its quiet blocks must leave those
records alone. `ignored` and `recorded` put a filter first for the length
of the block and take it out again. A warning raised meanwhile obeys the
filter, and the records do not notice. matplotlib's own ``catch_warnings``
blocks stay as they are.

Under context-aware warnings (``sys.flags.context_aware_warnings``), where
the filters belong to the context, both fall back to `catch_warnings`.
"""

import contextlib
import sys
import warnings


def _context_aware():
    """Whether the filters belong to the context (Python 3.14's
    ``-X context_aware_warnings``)."""
    return getattr(sys.flags, "context_aware_warnings", False)


@contextlib.contextmanager
def _filtered(action, category):
    """While the filter *action* for *category* comes first."""
    entry = (action, None, category, None, 0)
    filters = warnings.filters
    filters.insert(0, entry)
    try:
        yield
    finally:
        try:
            filters.remove(entry)
        except ValueError:
            # The program replaced or cleared the filters meanwhile.
            pass


@contextlib.contextmanager
def ignored(category=Warning):
    """While warnings of *category* are ignored."""
    if _context_aware():
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", category)
            yield
        return
    with _filtered("ignore", category):
        yield


@contextlib.contextmanager
def recorded():
    """While every warning is recorded, each as it comes, in the list
    given (of `warnings.WarningMessage`), and not shown. A warning a
    module has already shown once at a line is not given again, as the
    default filters keep it back there."""
    if _context_aware():
        with warnings.catch_warnings(record=True) as log:
            warnings.simplefilter("always")
            yield log
        return
    log = []
    module = warnings
    shown, implementation = module.showwarning, module._showwarnmsg_impl
    module.showwarning = module._showwarning_orig
    module._showwarnmsg_impl = log.append
    try:
        with _filtered("always", Warning):
            yield log
    finally:
        module.showwarning, module._showwarnmsg_impl = shown, implementation
