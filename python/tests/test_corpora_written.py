"""
The differential tester's corpora of the repository's own
(``spec/diff/corpora.py``): a manifest entry with a ``path`` instead of a
repository, whose programs run from the repository's directory, unfetched.
"""

import importlib.util
import pathlib
import sys

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]


def _load(name, file):
    """The module *file* of ``spec/diff``, loaded on its own as *name*."""
    spec = importlib.util.spec_from_file_location(name, ROOT / "spec/diff" / file)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def corpora():
    """``spec/diff/corpora.py`` and the module of its directory it imports,
    loaded on their own (the directory is not put on ``sys.path``: it holds
    modules named as the standard library's)."""
    try:
        _load("doc_examples", "doc_examples.py")
        yield _load("diff_corpora", "corpora.py")
    finally:
        for name in ("doc_examples", "diff_corpora"):
            sys.modules.pop(name, None)


def _manifest(tmp_path, body):
    manifest = tmp_path / "corpora.toml"
    manifest.write_text(body)
    return manifest


@pytest.mark.parametrize("name", ["written", "written2"])
def test_the_written_families_are_corpora_of_the_repository(corpora, name):
    written = next(corpus for corpus in corpora.load() if corpus.name == name)
    assert written.path == f"spec/diff/{name}"
    assert (written.repository, written.commit, written.fetch, written.sha256) == ("", "", (), "")
    assert written.tree(pathlib.Path("/nowhere")) == ROOT / "spec/diff" / name


def test_a_path_corpus_runs_its_files_unfetched(corpora, tmp_path):
    family = tmp_path / "repo" / "family"
    (family / "bars").mkdir(parents=True)
    (family / "bars" / "grouped.py").write_text("print('bars')\n")
    (family / "bars" / "notes.txt").write_text("not a program\n")
    (family / "links").mkdir()
    (family / "links" / "svg_urls.py").write_text("bar.set_url('https://example.org/a')\n")
    corpora.ROOT, root = tmp_path / "repo", corpora.ROOT
    try:
        [corpus] = corpora.load(_manifest(tmp_path, (
            '[family]\ntitle = "t"\npath = "family"\nlicence = "MIT"\n'
            'programs = ["*/*.py"]\noutputs = ["figures"]\n')))
        out = tmp_path / "out"
        program, linked = corpora.programs(corpus, out)
        assert program.name == "corpus/family/bars/grouped"
        assert linked.name == "corpus/family/links/svg_urls", "a URL is data, not a download"
        assert (program.path, program.mode) == (family / "bars" / "grouped.py", "program")
        assert not (out / "corpora").exists(), "nothing is fetched or converted"
        placed = program.lay_out(tmp_path / "run")
        assert placed.read_text() == "print('bars')\n"
        assert (tmp_path / "run" / "bars" / "notes.txt").is_symlink()
        assert (tmp_path / "run" / "figures").is_dir()
    finally:
        corpora.ROOT = root


def test_a_path_corpus_without_its_directory_is_refused(corpora, tmp_path):
    [corpus] = corpora.load(_manifest(tmp_path, (
        '[gone]\ntitle = "t"\npath = "spec/diff/no-such-family"\nlicence = "MIT"\n'
        'programs = ["*.py"]\n')))
    with pytest.raises(corpora.CorpusError, match="no directory spec/diff/no-such-family"):
        corpora.programs(corpus, tmp_path)


def _family(corpora, tmp_path, files, programs):
    """The programs of a path corpus of *files* (relative name: text) whose
    ``programs`` globs are *programs*."""
    family = tmp_path / "repo" / "family"
    for relative, text in files.items():
        (family / relative).parent.mkdir(parents=True, exist_ok=True)
        (family / relative).write_text(text)
    corpora.ROOT, root = tmp_path / "repo", corpora.ROOT
    try:
        [corpus] = corpora.load(_manifest(tmp_path, (
            '[family]\ntitle = "t"\npath = "family"\nlicence = "MIT"\n'
            f'programs = {list(programs)!r}\n'.replace("'", '"'))))
        return corpora.programs(corpus, tmp_path / "out")
    finally:
        corpora.ROOT = root


_NOTEBOOK = '{"cells": [{"cell_type": "code", "source": ["print(2)"]}], "nbformat": 4}'


def test_a_page_beside_a_script_of_its_name_keeps_its_suffix(corpora, tmp_path):
    # python-control's examples/repr_gallery.py and repr_gallery.ipynb.
    script, page = _family(corpora, tmp_path, {"ex/gallery.py": "print(1)\n", "ex/gallery.ipynb": _NOTEBOOK},
                           ["ex/*.py", "ex/*.ipynb"])
    assert (script.name, script.relative, script.mode) == ("corpus/family/ex/gallery", "ex/gallery.py", "program")
    assert (page.name, page.relative, page.mode) == ("corpus/family/ex/gallery.ipynb", "ex/gallery.ipynb.py", "cells")
    placed = page.lay_out(tmp_path / "run")
    assert placed == tmp_path / "run" / "ex" / "gallery.ipynb.py"
    assert (tmp_path / "run" / "ex" / "gallery.py").read_text() == "print(1)\n", "the script stays itself"


def test_a_page_never_stands_in_its_scripts_place(corpora, tmp_path):
    # probably-overthinking-it's redline.ipynb imports its redline.py.
    [page] = _family(corpora, tmp_path, {"redline.ipynb": _NOTEBOOK, "redline.py": "SPEED = 1\n"},
                     ["*.ipynb"])
    assert (page.name, page.relative) == ("corpus/family/redline.ipynb", "redline.ipynb.py")
    page.lay_out(tmp_path / "run")
    assert (tmp_path / "run" / "redline.py").read_text() == "SPEED = 1\n"


def test_a_page_alone_takes_the_name_of_its_source(corpora, tmp_path):
    [page] = _family(corpora, tmp_path, {"ex/gallery.ipynb": _NOTEBOOK}, ["ex/*.ipynb"])
    assert (page.name, page.relative) == ("corpus/family/ex/gallery", "ex/gallery.py")


def test_two_programs_of_one_name_are_refused(corpora, tmp_path):
    page = "```python\nprint(3)\n```\n"
    with pytest.raises(corpora.CorpusError, match=r"would both run as corpus/family/ex/notes"):
        _family(corpora, tmp_path, {"ex/notes.md": page, "ex/notes.rst": ".. plot::\n\n   print(4)\n"},
                ["ex/*.md", "ex/*.rst"])
