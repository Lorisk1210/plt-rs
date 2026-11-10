"""
Docstring helpers: %-substitution of shared snippets (``interpd``) and
docstring copying.

Kept from matplotlib 3.11.2 under the Matplotlib License. pltrs adds
`cleandoc`, which cleans the docstrings Python compiles quicker than
`inspect.cleandoc`, and keeps the ``ClassName:kwdoc`` tables of its own
artist classes between runs (`_KwdocTables`): together they are most of
the time ``import matplotlib.pyplot`` spends on docstrings. The same file
keeps the code of the functions the package makes from source text as it
is imported (`compiled`).
"""

import atexit
import contextlib
import inspect
import marshal
import os
import re
import sys
import types
import zlib

from . import _api

#: A line after the first that starts with neither a space nor a line
#: break: the text has no indentation common to those lines.
_UNINDENTED_LINE = re.compile(r"\n[^ \n]")


def cleandoc(doc):
    """
    `inspect.cleandoc` of *doc*. Python 3.13 and later compile docstrings
    without the indentation their lines after the first share, which leaves
    only the first line's and the blank lines around to strip: done here
    without going through the lines one by one.
    """
    if "\t" in doc or not _UNINDENTED_LINE.search(doc):
        return inspect.cleandoc(doc)
    first, newline, rest = doc.partition("\n")
    return (first.lstrip(" ") + newline + rest).strip("\n")


def kwarg_doc(text):
    """
    Decorator for defining the kwdoc documentation of artist properties.

    This decorator can be applied to artist property setter methods.
    The given text is stored in a private attribute ``_kwarg_doc`` on
    the method.  It is used to overwrite auto-generated documentation
    in the *kwdoc list* for artists. The kwdoc list is used to document
    ``**kwargs`` when they are properties of an artist. See e.g. the
    ``**kwargs`` section in `.Axes.text`.

    The text should contain the supported types, as well as the default
    value if applicable, e.g.:

        @_docstring.kwarg_doc("bool, default: :rc:`text.usetex`")
        def set_usetex(self, usetex):

    See Also
    --------
    matplotlib.artist.kwdoc

    """
    def decorator(func):
        func._kwarg_doc = text
        return func
    return decorator


class Substitution:
    """
    A decorator that performs %-substitution on an object's docstring.

    This decorator should be robust even if ``obj.__doc__`` is None (for
    example, if -OO was passed to the interpreter).

    Usage: construct a docstring.Substitution with a sequence or dictionary
    suitable for performing substitution; then decorate a suitable function
    with the constructed object, e.g.::

        sub_author_name = Substitution(author='Jason')

        @sub_author_name
        def some_function(x):
            "%(author)s wrote this function"

        # note that some_function.__doc__ is now "Jason wrote this function"

    One can also use positional arguments::

        sub_first_last_names = Substitution('Edgar Allen', 'Poe')

        @sub_first_last_names
        def some_function(x):
            "%s %s wrote the Raven"
    """
    def __init__(self, *args, **kwargs):
        if args and kwargs:
            raise TypeError("Only positional or keyword args are allowed")
        self.params = args or kwargs

    def __call__(self, func):
        if func.__doc__:
            func.__doc__ = cleandoc(func.__doc__) % self.params
        return func


class _ArtistKwdocLoader(dict):
    """The substitutions, loading ``ClassName:kwdoc`` ones on first use."""

    def __missing__(self, key):
        if not key.endswith(":kwdoc"):
            raise KeyError(key)
        name = key[:-len(":kwdoc")]
        from .artist import Artist, kwdoc
        try:
            cls, = (cls for cls in _api.recursive_subclasses(Artist)
                    if cls.__name__ == name)
        except ValueError as e:
            raise KeyError(key) from e
        return self.setdefault(key, _kwdoc_tables.get(cls, kwdoc))


class _ArtistPropertiesSubstitution:
    """
    A class to substitute formatted placeholders in docstrings.

    This is realized in a single instance ``_docstring.interpd``.

    Use `~._ArtistPropertiesSubstition.register` to define placeholders and
    their substitution, e.g. ``_docstring.interpd.register(name="some value")``.

    Use this as a decorator to apply the substitution::

        @_docstring.interpd
        def some_func():
            '''Replace %(name)s.'''

    Decorating a class triggers substitution both on the class docstring and
    on the class' ``__init__`` docstring (which is a commonly required
    pattern for Artist subclasses).

    Substitutions of the form ``%(classname:kwdoc)s`` (ending with the
    literal ":kwdoc" suffix) trigger lookup of an Artist subclass with the
    given *classname*, and are substituted with the `.kwdoc` of that class.
    """

    def __init__(self):
        self.params = _ArtistKwdocLoader()

    def register(self, **kwargs):
        """
        Register substitutions.

        ``_docstring.interpd.register(name="some value")`` makes "name" available
        as a named parameter that will be replaced by "some value".
        """
        self.params.update(**kwargs)

    def __call__(self, obj):
        if obj.__doc__:
            obj.__doc__ = cleandoc(obj.__doc__) % self.params
        if isinstance(obj, type) and obj.__init__ != object.__init__:
            self(obj.__init__)
        return obj


def copy(source):
    """Copy a docstring from another source function (if present)."""
    def do_copy(target):
        if source.__doc__:
            target.__doc__ = source.__doc__
        return target
    return do_copy


# Create a decorator that will house the various docstring snippets reused
# throughout Matplotlib.
interpd = _ArtistPropertiesSubstitution()


class _KwdocTables:
    """
    The ``ClassName:kwdoc`` tables of the package's own artist classes,
    and the code of the functions its modules make from source text as
    they are imported (`code`: pyplot's wrappers, the constructors of the
    patch styles), kept between runs in a file of the cache directory, as
    Python keeps compiled modules: what is written there is used while no
    module of the package, nor its extension, has changed size or
    modification time since, and is computed again (and written when
    Python exits) otherwise. A table depends on the class and on
    ``docstring.hardcopy`` alone, code on its source and file name.
    """

    #: The version of the file's format.
    _FORMAT = 2

    def __init__(self):
        #: The tables by class and ``docstring.hardcopy``, once read.
        self._tables = None
        #: What the file must have been written for: the format, Python,
        #: the package and the digest of the sizes and times of its files.
        self._stamp = None
        #: Whether a table was computed that the file lacks.
        self._changed = False

    def get(self, cls, kwdoc):
        """The table of the artist class *cls*: ``kwdoc(cls)``, kept."""
        module = sys.modules.get(cls.__module__)
        if not _in_package(getattr(module, "__file__", None)):
            return kwdoc(cls)
        from . import rcParams
        key = f"{cls.__module__}.{cls.__qualname__}:{rcParams['docstring.hardcopy']}"
        tables = self._read()
        table = tables.get(key)
        if type(table) is not str:
            table = tables[key] = kwdoc(cls)
            self._keep()
        return table

    def code(self, source, filename):
        """``compile(source, filename, "exec")``, kept (by the source's
        digest: the sources are not kept)."""
        tables = self._read()
        key = ("code", filename, _digest(source.encode()))
        code = tables.get(key)
        if type(code) is not types.CodeType:
            code = tables[key] = compile(source, filename, "exec")
            self._keep()
        return code

    def _keep(self):
        """Write the file when Python exits, now that it lacks something."""
        if not self._changed:
            self._changed = True
            atexit.register(self._write)

    def _read(self):
        """The tables of the file, if it was written for this package as it
        is, else none."""
        if self._tables is None:
            self._tables = {}
            try:
                self._stamp = (self._FORMAT, sys.version, sys.flags.optimize,
                               _PACKAGE, _digest(marshal.dumps(_package_files())))
                with open(_cache_file(), "rb") as file:
                    stamp, tables = marshal.load(file)
            except (OSError, EOFError, ValueError, TypeError):
                return self._tables
            if stamp == self._stamp and isinstance(tables, dict):
                self._tables = tables
        return self._tables

    def _write(self):
        """Write the tables to the file, replacing it; nothing when the
        package's files or the cache directory cannot be read or written."""
        if self._stamp is None:
            return
        try:
            path = _cache_file()
        except OSError:
            return
        written = f"{path}.{os.getpid()}"
        try:
            with open(written, "wb") as file:
                marshal.dump((self._stamp, self._tables), file)
            os.replace(written, path)
        except OSError:
            with contextlib.suppress(OSError):
                os.remove(written)


#: The directory of the package.
_PACKAGE = os.path.dirname(os.path.abspath(__file__))


def _in_package(path):
    """Whether the file *path* (if any) belongs to the package."""
    return path is not None and os.path.abspath(path).startswith(_PACKAGE + os.sep)


def _package_files():
    """The modules of the package (in it and in its subdirectories named as
    packages) and its extension, as ``(path, size, modification time)``
    tuples, the paths of the modules relative to the package."""
    from . import _native
    status = os.stat(_native.__file__)
    files = [(_native.__file__, status.st_size, status.st_mtime_ns)]
    pending = [""]
    while pending:
        directory = pending.pop()
        with os.scandir(os.path.join(_PACKAGE, directory)) as entries:
            for entry in entries:
                if entry.is_dir(follow_symlinks=False):
                    if entry.name.isidentifier() and entry.name != "__pycache__":
                        pending.append(os.path.join(directory, entry.name))
                elif entry.name.endswith(".py"):
                    status = entry.stat()
                    files.append((os.path.join(directory, entry.name),
                                  status.st_size, status.st_mtime_ns))
    return tuple(sorted(files))


def _digest(data):
    """A digest of the bytes *data*: their length and two checksums, which
    differ for all but a vanishing share of different data."""
    return len(data), zlib.crc32(data), zlib.adler32(data)


def _cache_file():
    """The file of the cache directory holding the tables, one for each
    directory the package is in."""
    from ._native import cbook
    name = f"kwdoc-v{_KwdocTables._FORMAT}-{zlib.crc32(os.fsencode(_PACKAGE)):08x}.pltrs"
    return os.path.join(cbook.cache_dir(), name)


_kwdoc_tables = _KwdocTables()


def compiled(source, filename):
    """``compile(source, filename, "exec")`` for a module of the package,
    kept between runs (`_KwdocTables.code`)."""
    return _kwdoc_tables.code(source, filename)
