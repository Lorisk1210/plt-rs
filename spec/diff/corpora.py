"""The corpora of real-world matplotlib programs, for the differential tester.

``corpora.toml`` lists them (see its header). ``driver.py --suite corpus``
uses this module to

1. fetch each corpus (``fetch``): a sparse, shallow git checkout of its
   ``fetch`` patterns at its commit into ``OUT/corpora/<name>/tree``,
   checked against the manifest's SHA-256 (``tree_digest``) and made
   read-only, done again only when the manifest changes;
2. list its programs (``programs``): the files its ``programs`` globs
   match, but those its ``exclude`` globs match and those naming a URL
   outside its ``data-urls`` (they would need the network); a notebook, a
   Markdown page or a reStructuredText page (its IPython directives)
   becomes a script of cells (``# %%`` lines, the percent format), the
   corpus' ``prelude`` its first, under ``OUT/corpora/<name>/converted``,
   as does a script whose URLs of the repository's files are rewritten
   into the tree (as paths, or as ``file:`` URLs for the programs that
   download them);
3. lay out the tree a run sees (``shadow``): the tree's directories, its
   files as links into it, and the ``outputs`` directories empty (with the
   subdirectories their repository has, which ``fetch`` lists from git's
   trees without fetching their files), so that a program reads its
   repository's files where it expects them and writes its figures without
   touching the tree; a figure a script saves next to itself under its own
   name (a book's committed output, ``plot.png`` of ``plot.py``) is left
   out, for the script to save it anew.

It uses the standard library and git alone.
"""

from __future__ import annotations

import ast
import dataclasses
import fnmatch
import hashlib
import json
import os
import pathlib
import re
import shutil
import stat
import subprocess
import textwrap
import tomllib
from typing import Any, Iterator

HERE = pathlib.Path(__file__).resolve().parent
#: The manifest of the corpora.
MANIFEST = HERE / "corpora.toml"
#: The line starting a cell of a script in the percent format.
CELL_MARK = "# %%"
#: The suffixes of programs converted to scripts of cells.
CELL_SUFFIXES = (".ipynb", ".md", ".rst")

#: The suffixes of the files matplotlib saves figures as.
FIGURE_SUFFIXES = frozenset({
    ".png", ".pdf", ".svg", ".svgz", ".eps", ".ps", ".pgf",
    ".jpg", ".jpeg", ".gif", ".tif", ".tiff", ".webp", ".avif"})
#: A Markdown page's blocks of Python code.
_MARKDOWN_CODE = re.compile(r"^```(?:python|py)[ \t]*\n(.*?)^```", re.M | re.S)
#: A MyST page's code cells (the blocks Jupyter runs, its other blocks only
#: shown), each with its kernel's name.
_MYST_CODE = re.compile(r"^```\{code-cell\}(?:[ \t]+[\w-]+)?[ \t]*\n(.*?)^```", re.M | re.S)
#: The options opening a MyST code cell: ``:key: value`` lines, or a YAML
#: block between ``---`` lines.
_MYST_OPTIONS = re.compile(r"\A(?:---\n.*?^---\n|(?:^:[\w-]+:.*\n)+)", re.M | re.S)
#: A reStructuredText page's IPython directive (``.. ipython:: python``):
#: its options and code, the lines indented under it (or blank).
_RST_IPYTHON = re.compile(
    r"^(?P<indent>[ \t]*)\.\. ipython::[ \t]+python[ \t]*\n(?P<body>(?:(?P=indent)[ \t]+\S.*\n|[ \t]*\n)*)", re.M)
#: The options opening an IPython directive's block.
_RST_OPTIONS = re.compile(r"\A(?:[ \t]*:[\w-]+:.*\n)*")
#: The IPython directive's pseudo-decorators (``@savefig name.png``,
#: ``@suppress``...): no Python, the line after them runs as it is.
_RST_DECORATOR = re.compile(r"^\s*@(?:savefig|suppress|doctest|okexcept|okwarning)\b.*$", re.M)
#: An IPython line magic, shell escape or help request: no Python.
_IPYTHON_LINE = re.compile(r"^\s*(?:[%!]|[\w.]+\?{1,2}\s*$)")
#: A string naming a URL.
_URL = re.compile(r"https?://", re.I)


class CorpusError(Exception):
    """A corpus that cannot be fetched or does not match its manifest."""


@dataclasses.dataclass(frozen=True)
class Corpus:
    """One corpus of the manifest."""

    name: str
    title: str
    repository: str
    commit: str
    licence: str
    fetch: tuple[str, ...]
    sha256: str
    programs: tuple[str, ...]
    exclude: tuple[str, ...]
    outputs: tuple[str, ...]
    data_urls: tuple[str, ...]
    file_urls: tuple[str, ...]
    #: Code each program's cells start with, a cell of its own.
    prelude: str

    def directory(self, out: pathlib.Path) -> pathlib.Path:
        """Where the corpus' tree and converted programs go."""
        return out / "corpora" / self.name

    def tree(self, out: pathlib.Path) -> pathlib.Path:
        """The fetched tree."""
        return self.directory(out) / "tree"


def _output_listing(tree: pathlib.Path) -> pathlib.Path:
    """The file listing the output directories of the corpus fetched into
    *tree*, with their subdirectories."""
    return tree.parent / "outputs.json"


@dataclasses.dataclass(frozen=True)
class CorpusProgram:
    """One program of a corpus, as the driver runs it."""

    corpus: Corpus
    #: The corpus' fetched tree.
    tree: pathlib.Path
    #: The program's path in the tree (a converted program's: its source's,
    #: with ``.py`` for its suffix), where a run puts it.
    relative: str
    #: The text run: the file in the tree, or the converted script.
    path: pathlib.Path
    #: ``program`` (run whole) or ``cells`` (cell by cell).
    mode: str

    @property
    def name(self) -> str:
        """The program's name in the report."""
        return f"corpus/{self.corpus.name}/{self.relative.removesuffix('.py')}"

    def lay_out(self, into: pathlib.Path) -> pathlib.Path:
        """Lays out the tree a run of the program sees at *into* (``shadow``)
        and puts the program's text at its place there, a file of its own
        (so that its ``__file__`` names the copy); returns that place."""
        shadow(self.corpus, self.tree, into)
        program = into / self.relative
        program.unlink(missing_ok=True)
        program.parent.mkdir(parents=True, exist_ok=True)
        program.write_bytes(self.path.read_bytes())
        return program


def load(manifest: pathlib.Path = MANIFEST) -> list[Corpus]:
    """The corpora of the manifest, in its order."""
    tables = tomllib.loads(manifest.read_text())
    return [
        Corpus(
            name=name,
            title=table["title"],
            repository=table["repository"],
            commit=table["commit"],
            licence=table["licence"],
            fetch=tuple(table["fetch"]),
            sha256=table["sha256"],
            programs=tuple(table["programs"]),
            exclude=tuple(table.get("exclude", ())),
            outputs=tuple(table.get("outputs", ())),
            data_urls=tuple(table.get("data-urls", ())),
            file_urls=tuple(table.get("file-urls", ())),
            prelude=table.get("prelude", ""),
        )
        for name, table in tables.items()
    ]


def _git(*args: str | os.PathLike[str]) -> str:
    """What git prints running *args*."""
    done = subprocess.run(["git", *map(str, args)], capture_output=True, text=True)
    if done.returncode != 0:
        raise CorpusError(f"git {' '.join(map(str, args[:3]))}...: {done.stderr.strip()[-500:]}")
    return done.stdout


def _files(tree: pathlib.Path) -> Iterator[pathlib.Path]:
    """The tree's files (git's own aside), as paths relative to it."""
    for directory, subdirectories, files in os.walk(tree):
        subdirectories[:] = sorted(d for d in subdirectories if d != ".git")
        here = pathlib.Path(directory).relative_to(tree)
        for name in files:
            if not (here == pathlib.Path(".") and name == ".git"):
                yield here / name


def tree_digest(tree: pathlib.Path) -> str:
    """The SHA-256 of the ``sha256sum`` lines of every regular file of
    *tree*, sorted by path (bytewise, as the C locale sorts them); symbolic
    links, which ``find -type f`` leaves out too, are not hashed."""
    paths = sorted((f"./{relative.as_posix()}" for relative in _files(tree)
                    if not (tree / relative).is_symlink()), key=str.encode)
    listing = "".join(f"{hashlib.sha256((tree / path).read_bytes()).hexdigest()}  {path}\n" for path in paths)
    return hashlib.sha256(listing.encode()).hexdigest()


def fetch(corpus: Corpus, out: pathlib.Path) -> pathlib.Path:
    """The corpus' tree, fetched first unless it is there for the manifest's
    entry as it is."""
    directory = corpus.directory(out)
    tree = corpus.tree(out)
    stamp = directory / "fetched.json"
    entry = json.dumps(dataclasses.asdict(corpus), sort_keys=True)
    if stamp.exists() and stamp.read_text() == entry and tree.is_dir():
        return tree
    if directory.exists():
        shutil.rmtree(directory, onexc=_writable_then_retry)
    directory.mkdir(parents=True)
    _git("init", "--quiet", f"--separate-git-dir={directory / 'git'}", tree)
    _git("-C", tree, "remote", "add", "origin", corpus.repository)
    _git("-C", tree, "sparse-checkout", "set", "--no-cone", *corpus.fetch)
    _git("-C", tree, "fetch", "--quiet", "--depth", "1", "--filter=blob:none", "origin", corpus.commit)
    _git("-C", tree, "-c", "advice.detachedHead=false", "checkout", "--quiet", "FETCH_HEAD")
    digest = tree_digest(tree)
    if digest != corpus.sha256:
        raise CorpusError(f"{corpus.name}: the tree fetched has SHA-256 {digest}, "
                          f"the manifest says {corpus.sha256 or 'nothing'}")
    outputs = [*corpus.outputs]
    if corpus.outputs:
        outputs += _git("-C", tree, "ls-tree", "-r", "-d", "--name-only", "HEAD", "--", *corpus.outputs).split()
    _output_listing(tree).write_text(json.dumps(sorted(set(outputs))))
    for relative in _files(tree):
        path = tree / relative
        if not path.is_symlink():
            path.chmod(path.stat().st_mode & ~(stat.S_IWUSR | stat.S_IWGRP | stat.S_IWOTH))
    stamp.write_text(entry)
    return tree


def _writable_then_retry(function: Any, path: str, _error: BaseException) -> None:
    """Removes a read-only file of a tree fetched before (``rmtree``'s
    ``onexc``)."""
    os.chmod(os.path.dirname(path), stat.S_IRWXU)
    os.chmod(path, stat.S_IRWXU)
    function(path)


def _matches(relative: str, patterns: tuple[str, ...]) -> bool:
    return any(fnmatch.fnmatchcase(relative, pattern) for pattern in patterns)


def _string_sum(node: ast.AST) -> str | None:
    """The string a literal or a sum of literals spells, else None."""
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Add):
        left, right = _string_sum(node.left), _string_sum(node.right)
        if left is not None and right is not None:
            return left + right
    return None


def _join_url_sums(text: str, prefixes: tuple[str, ...]) -> str:
    """*text* with every sum of string literals spelling a URL that starts
    with one of *prefixes* (``'https://host/' + 'file'``, a program keeping
    its lines short) written as one literal, so that the URL is rewritten
    whole; the lines the sum spanned are kept, as continuation lines."""
    if not prefixes:
        return text
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return text
    folded = tuple(prefix.lower() for prefix in prefixes)
    data = text.encode()
    starts = [0, *(index + 1 for index, byte in enumerate(data) if byte == ord("\n"))]
    spans: list[tuple[int, int, str]] = []
    for node in ast.walk(tree):
        value = _string_sum(node) if isinstance(node, ast.BinOp) else None
        if value is None or not value.lower().startswith(folded):
            continue
        start = starts[node.lineno - 1] + node.col_offset
        end = starts[node.end_lineno - 1] + node.end_col_offset
        if any(start >= outer[0] and end <= outer[1] for outer in spans):
            continue
        spans.append((start, end, repr(value) + "\\\n" * (node.end_lineno - node.lineno)))
    for start, end, literal in sorted(spans, reverse=True):
        data = data[:start] + literal.encode() + data[end:]
    return data.decode()


def _rewrite_urls(text: str, corpus: Corpus, tree: pathlib.Path) -> str:
    """*text* with the URLs of the repository's own files made paths into
    *tree* (``data-urls``) or ``file:`` URLs of them (``file-urls``)."""
    text = _join_url_sums(text, corpus.data_urls + corpus.file_urls)
    for prefixes, into in ((corpus.data_urls, tree.as_posix() + "/"), (corpus.file_urls, tree.as_uri() + "/")):
        for prefix in prefixes:
            text = re.sub(re.escape(prefix), lambda _, into=into: into, text, flags=re.I)
    return text


def _names_url(text: str) -> bool:
    """Whether the code *text* has a string naming a URL (outside comments
    and docstrings)."""
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return False
    docstrings = {
        id(node.body[0].value) for node in ast.walk(tree)
        if isinstance(node, (ast.Module, ast.FunctionDef, ast.ClassDef, ast.AsyncFunctionDef))
        and node.body and isinstance(node.body[0], ast.Expr) and isinstance(node.body[0].value, ast.Constant)
    }
    return any(
        isinstance(node, ast.Constant) and isinstance(node.value, str) and _URL.search(node.value)
        and id(node) not in docstrings
        for node in ast.walk(tree)
    )


def _bound_names(statements: list[ast.stmt]) -> set[str]:
    """The names the *statements* of a cell bind at its top level (not those
    of the functions and classes it defines)."""
    names: set[str] = set()
    pending: list[ast.AST] = list(statements)
    while pending:
        node = pending.pop()
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            names.add(node.name)
            continue
        if isinstance(node, (ast.Lambda, ast.ListComp, ast.SetComp, ast.DictComp, ast.GeneratorExp)):
            continue
        if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Store):
            names.add(node.id)
        elif isinstance(node, (ast.Import, ast.ImportFrom)):
            names.update((alias.asname or alias.name).partition(".")[0] for alias in node.names)
        elif isinstance(node, ast.ExceptHandler) and node.name:
            names.add(node.name)
        pending.extend(ast.iter_child_nodes(node))
    names.discard("*")
    return names


def _awaiting(code: str) -> str | None:
    """A cell awaiting at its top level, which IPython's kernels run as a
    coroutine (JupyterLite's ``await piplite.install(...)`` in a notebook's
    first cell), as Python a script runs: the cell as a coroutine run to
    its end, the names it binds global, its ``import *`` lines before it.
    None for code that does not compile so either."""
    try:
        tree = compile(code, "<cell>", "exec", flags=ast.PyCF_ONLY_AST | ast.PyCF_ALLOW_TOP_LEVEL_AWAIT,
                       dont_inherit=True)
        compile(tree, "<cell>", "exec", flags=ast.PyCF_ALLOW_TOP_LEVEL_AWAIT, dont_inherit=True)
    except SyntaxError:
        return None
    # (`import *` is for a module's level only: the cell's stay there.)
    lines = code.splitlines()
    starred = [node for node in tree.body
               if isinstance(node, ast.ImportFrom) and any(alias.name == "*" for alias in node.names)]
    first = "".join(f"{ast.get_source_segment(code, node)}\n" for node in starred)
    for node in starred:
        lines[node.lineno - 1:node.end_lineno] = [""] * (node.end_lineno - node.lineno + 1)
    names = sorted(_bound_names([node for node in tree.body if node not in starred]))
    declared = f"    global {', '.join(names)}\n" if names else ""
    body = textwrap.indent("\n".join(lines), "    ")
    return (f"{first}async def _awaiting_cell():\n{declared}{body}\n\n\n"
            "__import__('asyncio').run(_awaiting_cell())\n")


def _cell_code(source: str) -> str | None:
    """A cell's Python: without IPython's magics, shell escapes and help
    requests, a cell awaiting at its top level as a coroutine run to its
    end (`_awaiting`); None for a cell magic's cell or code Python cannot
    parse."""
    if source.lstrip().startswith("%%"):
        return None
    code = "\n".join(line for line in source.splitlines() if not _IPYTHON_LINE.match(line))
    try:
        ast.parse(code)
    except SyntaxError:
        return None
    try:
        compile(code, "<cell>", "exec", dont_inherit=True)
    except SyntaxError:
        # (Any other code Python parses but does not compile stays as it
        # is: its program does not run, under either implementation.)
        return _awaiting(code) or code
    return code


def _notebook_sources(notebook: dict[str, Any]) -> list[str]:
    """The sources of a notebook's code cells: of its ``cells`` (nbformat 4)
    or of its worksheets' (nbformat 3, the source under ``input``)."""
    every = notebook.get("cells") or [cell for sheet in notebook.get("worksheets", []) for cell in sheet["cells"]]
    sources = (cell.get("source", cell.get("input", "")) for cell in every if cell["cell_type"] == "code")
    return ["".join(source) if isinstance(source, list) else source for source in sources]


def _rst_sources(text: str) -> list[str]:
    """The code of a reStructuredText page's IPython directives, as Sphinx'
    ``ipython`` directive runs them: their options and pseudo-decorators
    left out, those shown only (``:verbatim:``) too."""
    sources = []
    for match in _RST_IPYTHON.finditer(text):
        body = textwrap.dedent(match["body"])
        options = _RST_OPTIONS.match(body)[0]
        if ":verbatim:" not in options:
            sources.append(_RST_DECORATOR.sub("", body[len(options):]))
    return sources


def cells(path: pathlib.Path) -> list[str]:
    """The code cells of a notebook, of a MyST page (its ``{code-cell}``
    blocks, their options left out), of a Markdown page's Python blocks or
    of a reStructuredText page's IPython directives."""
    if path.suffix == ".ipynb":
        sources = _notebook_sources(json.loads(path.read_text()))
    elif path.suffix == ".rst":
        sources = _rst_sources(path.read_text())
    else:
        text = path.read_text()
        sources = ([_MYST_OPTIONS.sub("", block, count=1) for block in _MYST_CODE.findall(text)]
                   or _MARKDOWN_CODE.findall(text))
    return [code for code in map(_cell_code, sources) if code is not None and code.strip()]


def as_script(code_cells: list[str]) -> str:
    """The cells as one script in the percent format."""
    return "".join(f"{CELL_MARK} [{index}]\n{code.rstrip()}\n\n" for index, code in enumerate(code_cells, 1))


def programs(corpus: Corpus, out: pathlib.Path) -> list[CorpusProgram]:
    """The corpus' programs (fetched first), converted where they need it."""
    tree = fetch(corpus, out)
    converted = corpus.directory(out) / "converted"
    found = []
    for pattern in corpus.programs:
        for path in sorted(tree.glob(pattern)):
            relative = path.relative_to(tree).as_posix()
            if _matches(relative, corpus.exclude):
                continue
            cellwise = path.suffix in CELL_SUFFIXES
            text = (as_script([corpus.prelude] * bool(corpus.prelude) + cells(path))
                    if cellwise else path.read_text())
            rewritten = _rewrite_urls(text, corpus, tree)
            if _names_url(rewritten):
                continue
            run_as = relative.removesuffix(path.suffix) + ".py"
            if cellwise or rewritten != text:
                path = converted / run_as
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(rewritten)
            found.append(CorpusProgram(corpus, tree, run_as, path, "cells" if cellwise else "program"))
    return found


def shadow(corpus: Corpus, tree: pathlib.Path, into: pathlib.Path) -> None:
    """Lays out at *into* the tree a run sees: *tree*'s directories, its
    files as links to them (copies where the system makes no links) but
    the figures its scripts save next to themselves under their own names,
    and the corpus' output directories empty, with their
    subdirectories."""
    if into.exists():
        shutil.rmtree(into)
    scripts = {path.relative_to(tree) for pattern in corpus.programs
               for path in tree.glob(pattern) if path.suffix == ".py"}
    for relative in _files(tree):
        if _matches(relative.as_posix(), tuple(f"{output}/*" for output in corpus.outputs)):
            continue
        if relative.suffix in FIGURE_SUFFIXES and relative.with_suffix(".py") in scripts:
            # The script's own figure, which it saves anew.
            continue
        target = into / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        try:
            target.symlink_to(tree / relative)
        except OSError:
            shutil.copyfile(tree / relative, target)
    for output in json.loads(_output_listing(tree).read_text()):
        (into / output).mkdir(parents=True, exist_ok=True)
