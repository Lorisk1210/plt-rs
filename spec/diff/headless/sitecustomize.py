"""Keeps the differential tester's programs off the screen.

The tester runs on Linux without a display: there the GUI toolkits fail
as they start (Tk's ``no display name``, Qt's missing platform),
matplotlib finds no valid display (its interactive backends refuse to
load, as ``'headless' is currently running``) and a program opening a web
browser finds none, so that matplotlib cannot run such a program and the
tester skips it. macOS and Windows always have a screen, the user's, so
the driver puts this directory first on ``PYTHONPATH`` there; Python
imports ``sitecustomize`` as every interpreter starts (the programs' own
child processes included), and this one makes such a system look the same
to a program, under matplotlib and under pltrs (Python) alike:

* ``display_is_valid()`` of both implementations' ``_c_internal_utils`` is
  False;
* Tk refuses to make a window, with the error it gives on Linux without a
  display (a Tcl interpreter alone it still makes);
* Qt draws on its offscreen platform, never on the screen;
* ``webbrowser`` finds no browser to open;
* a server listening on every interface listens on the loopback one only,
  as the programs run offline.

Each module is changed when a program first imports it, not before, so
that what a program imports stays its own.
"""

import importlib.abc
import os
import sys

#: Tk's error on Linux without a display.
NO_DISPLAY = "no display name and no $DISPLAY environment variable"
#: The position of ``_tkinter.create``'s ``useTk`` argument.
USE_TK = 5
#: The addresses meaning every interface, and the loopback address taking
#: their place, per address family.
EVERY_INTERFACE = {"": "127.0.0.1", "0.0.0.0": "127.0.0.1", "::": "::1"}


def _no_display(module):
    """``_c_internal_utils``: no display is valid."""
    module.display_is_valid = lambda: False


def _no_tk_window(module):
    """``_tkinter``: an interpreter with Tk fails as without a display."""
    create = module.create

    def create_without_tk(*args, **kwargs):
        use_tk = args[USE_TK] if len(args) > USE_TK else kwargs.get("useTk", True)
        if use_tk:
            raise module.TclError(NO_DISPLAY)
        return create(*args, **kwargs)

    module.create = create_without_tk


def _no_browser(module):
    """``webbrowser``: no browser is found, none opens."""
    def get(using=None):
        raise module.Error("could not locate runnable browser")

    module.get = get
    for name in ("open", "open_new", "open_new_tab"):
        setattr(module, name, lambda url, *args, **kwargs: False)


def _loopback_only(module):
    """``socket``: a socket bound to every interface binds to the loopback
    one."""
    bind = module.socket.bind

    def bind_loopback(self, address):
        if isinstance(address, tuple) and address and address[0] in EVERY_INTERFACE:
            address = (EVERY_INTERFACE[address[0]], *address[1:])
        return bind(self, address)

    module.socket.bind = bind_loopback


#: The change each module gets once imported.
CHANGES = {
    "matplotlib._c_internal_utils": _no_display,
    "pltrs._c_internal_utils": _no_display,
    "_tkinter": _no_tk_window,
    "webbrowser": _no_browser,
    "socket": _loopback_only,
}


class _ChangingLoader(importlib.abc.Loader):
    """A module's own loader, with the module changed once executed."""

    def __init__(self, loader, change):
        self.loader = loader
        self.change = change

    def create_module(self, spec):
        """The module, as its own loader creates it."""
        return self.loader.create_module(spec)

    def exec_module(self, module):
        """Executes the module with its own loader, then changes it."""
        self.loader.exec_module(module)
        self.change(module)

    def __getattr__(self, name):
        """The rest of the own loader's methods (``get_source``, ...)."""
        return getattr(self.loader, name)


class _ChangingFinder(importlib.abc.MetaPathFinder):
    """Finds the modules of `CHANGES` as the other finders do, giving them
    a loader that changes them."""

    def find_spec(self, name, path=None, target=None):
        """The module's spec, its loader changing it; None for the rest."""
        change = CHANGES.get(name)
        if change is None:
            return None
        for finder in sys.meta_path:
            find = getattr(finder, "find_spec", None)
            spec = None if finder is self or find is None else find(name, path, target)
            if spec is not None:
                if spec.loader is not None:
                    spec.loader = _ChangingLoader(spec.loader, change)
                return spec
        return None


os.environ["QT_QPA_PLATFORM"] = "offscreen"
sys.meta_path.insert(0, _ChangingFinder())
