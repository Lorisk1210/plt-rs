"""
Helper functions for testing (matplotlib's ``testing``): the
configuration image tests run in, and running code in a subprocess.
"""

import locale
import logging
import os
import subprocess
import sys

from .. import _api, rcdefaults, rcParams, use

_log = logging.getLogger(__name__)


def set_font_settings_for_testing():
    """Use the fonts matplotlib's baseline images were made with."""
    rcParams['font.family'] = 'DejaVu Sans'
    rcParams['text.hinting'] = 'default'


def set_reproducibility_for_testing():
    """Make SVG ids reproducible."""
    rcParams['svg.hashsalt'] = 'matplotlib'


def setup():
    """Configure matplotlib (here pltrs) as matplotlib's tests expect."""
    try:
        locale.setlocale(locale.LC_ALL, 'en_US.UTF-8')
    except locale.Error:
        try:
            locale.setlocale(locale.LC_ALL, 'English_United States.1252')
        except locale.Error:
            _log.warning(
                "Could not set locale to English/United States. "
                "Some date-related tests may fail.")

    use('Agg')

    with _api.suppress_matplotlib_deprecation_warning():
        rcdefaults()  # Start with all defaults

    set_font_settings_for_testing()
    set_reproducibility_for_testing()


def is_ci_environment():
    """Whether the tests run on a continuous integration service."""
    ci_environment_variables = [
        'CI', 'CONTINUOUS_INTEGRATION', 'TRAVIS', 'CIRCLECI', 'JENKINS',
        'GITLAB_CI', 'GITHUB_ACTIONS', 'TEAMCITY_VERSION', 'TF_BUILD',
    ]
    return any(os.getenv(env_var) for env_var in ci_environment_variables)


def subprocess_run_for_testing(command, env=None, timeout=60, stdout=None,
                               stderr=None, check=False, text=True,
                               capture_output=False, **kwargs):
    """
    Create and run a subprocess (a thin wrapper around `subprocess.run`).
    """
    if capture_output:
        stdout = stderr = subprocess.PIPE
    try:
        proc = subprocess.run(
            command, env=env, timeout=timeout, check=check,
            stdout=stdout, stderr=stderr, text=text, **kwargs)
    except subprocess.CalledProcessError as e:
        if e.stdout:
            _log.error(f"Subprocess output:\n{e.stdout}")
        if e.stderr:
            _log.error(f"Subprocess error:\n{e.stderr}")
        raise e
    return proc


def subprocess_run_helper(func, *args, timeout, extra_env=None):
    """
    Run a function in a sub-process.

    Parameters
    ----------
    func : function
        The function to be run.  It must be in a module that is importable.
    *args : str
        Any additional command line arguments to be passed in
        the first argument to ``subprocess.run``.
    extra_env : dict[str, str]
        Any additional environment variables to be set for the subprocess.
    """
    if is_ci_environment():
        timeout *= 6
    target = func.__name__
    module = func.__module__
    file = func.__code__.co_filename
    return subprocess_run_for_testing(
        [
            sys.executable,
            "-c",
            f"import importlib.util;"
            f"_spec = importlib.util.spec_from_file_location({module!r}, {file!r});"
            f"_module = importlib.util.module_from_spec(_spec);"
            f"_spec.loader.exec_module(_module);"
            f"_module.{target}()",
            *args,
        ],
        env={**os.environ, "SOURCE_DATE_EPOCH": "0", **(extra_env or {})},
        timeout=timeout, check=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
    )
