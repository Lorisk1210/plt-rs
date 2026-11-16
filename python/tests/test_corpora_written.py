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


def test_the_written_family_is_a_corpus_of_the_repository(corpora):
    written = next(corpus for corpus in corpora.load() if corpus.name == "written")
    assert written.path == "spec/diff/written"
    assert (written.repository, written.commit, written.fetch, written.sha256) == ("", "", (), "")
    assert written.tree(pathlib.Path("/nowhere")) == ROOT / "spec/diff/written"


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
