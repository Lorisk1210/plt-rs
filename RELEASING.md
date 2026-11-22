# Releasing pltrs

A release publishes three things from one tagged commit:

* pltrs (Python) on PyPI: the source distribution and one wheel for each
  platform. The workflow `.github/workflows/wheels.yml` builds, tests
  and uploads them.
* pltrs (Rust) on crates.io: nine crates. You publish them with
  `cargo publish`.
* A GitHub release with all archives. It includes the wheel of the
  `matplotlib` alias package, which is not on PyPI.

The examples in this document use version `0.1.1`. Replace it with your
version.

A version on PyPI or crates.io is permanent. You cannot upload the same
version again, even after you delete it. The steps that publish are
marked PUBLISHES.

## Wheels

| Wheel | System |
|-------|--------|
| `manylinux_2_17_x86_64`, `manylinux_2_17_aarch64` | Linux with glibc 2.17 or later |
| `musllinux_1_2_x86_64`, `musllinux_1_2_aarch64` | Linux with musl 1.2 or later |
| `macosx_10_12_universal2` | macOS 10.12 or later on Intel, macOS 11 or later on Apple silicon |
| `win_amd64` | 64-bit Windows 10 or later |
| `win_arm64` | Windows on arm64, only if you request it |

All wheels have the tag `cp312-abi3`. One wheel is for CPython 3.12 and
all later versions.

## Crates

Publish the crates in this order, because each crate needs the crates
before it:

`pltrs-core`, `pltrs-codec`, `pltrs-font`, `pltrs-raster`,
`pltrs-platform`, `pltrs-text`, `pltrs-render`, `pltrs-ui`, `pltrs`.

The crates `pltrs-python`, `pltrs-spec`, `xtask` and `pltrs-examples`
have `publish = false`.

## Release procedure

Do the steps on Linux or macOS, in a clone where `origin` is the GitHub
repository. A Windows checkout can change line endings, and the crate
packages are then different from the tag.

### 1. Update the symbol order, if native code changed

Skip this step if no Rust code changed since the last release.

The file `crates/pltrs-python/symbol-order.txt` gives the link order of
the functions in the x86_64 manylinux wheel. A correct order decreases
the memory that each process uses. Run the tool two times, then commit
the file:

```sh
cargo xtask python --release
.venv-pltrs/bin/python crates/pltrs-python/tools/symbol_order.py
cargo xtask python --release
.venv-pltrs/bin/python crates/pltrs-python/tools/symbol_order.py
git add crates/pltrs-python/symbol-order.txt
git commit -m "pltrs-python: the symbol order of the release"
```

### 2. Set the version

`Cargo.toml` is the only source of the version. The crates, the wheels
and the alias package read it.

1. In `Cargo.toml`, set `version` in `[workspace.package]`.
2. In `Cargo.toml`, set the nine requirements in
   `[workspace.dependencies]` to the same version, for example
   `version = "=0.1.1"`.
3. Run `cargo check --workspace`. This updates `Cargo.lock`.
4. In `CHANGELOG.md`, add a section for the version with the date. Put
   the marker `<!--fact:version-->` in front of the version in its
   heading, and remove the marker from the previous heading.
5. In `README.md`, update the version of the alias package.

Then run the checks, commit and push `main`:

```sh
git switch main && git pull --ff-only
python python/tools/crate_notices.py --check
.venv/bin/python -m pytest python/tests/test_readme_facts.py
cargo publish --dry-run -p pltrs-core -p pltrs-codec -p pltrs-font -p pltrs-raster \
    -p pltrs-platform -p pltrs-text -p pltrs-render -p pltrs-ui -p pltrs
git commit -am "pltrs 0.1.1"
git push origin main
```

The dry run builds each crate from its package and uploads nothing.

### 3. Optional: run the workflow without publishing

A manual run builds and tests all archives. It publishes nothing. You
can download the archives from the run page.

```sh
gh workflow run wheels.yml --ref main
gh run watch --exit-status $(gh run list --workflow wheels.yml --event workflow_dispatch --limit 1 --json databaseId --jq '.[0].databaseId')
```

A full run uses about 10 hours of billed Actions minutes. Skip this step
if the tag run in step 4 is sufficient for you. If the tag run fails,
nothing is published. You then delete the tag and try again.

### 4. Push the tag

The tag starts the workflow.

```sh
git tag -a v0.1.1 -m "pltrs 0.1.1"
git push origin v0.1.1
gh run watch --exit-status $(gh run list --workflow wheels.yml --event push --limit 1 --json databaseId --jq '.[0].databaseId')
```

The workflow does these steps:

1. It makes sure that the tag is `v` plus the version in `Cargo.toml`.
2. It builds the source distribution, the alias wheel and all wheels.
3. It installs each archive on its platform and runs the tests.
4. It waits for your approval before it publishes.

### 5. Approve the upload to PyPI (PUBLISHES)

1. Open the run page: *Actions*, then *wheels*, then the run.
2. Click *Review deployments*.
3. Select `pypi`, then click *Approve and deploy*.

The job uploads the archives to PyPI through trusted publishing. Then it
creates a draft GitHub release with all archives.

### 6. Publish the crates (PUBLISHES)

Run the commands one at a time, from a clean checkout of the tag:

```sh
git switch --detach v0.1.1
git status --porcelain              # must print nothing
cargo publish -p pltrs-core
cargo publish -p pltrs-codec
cargo publish -p pltrs-font
cargo publish -p pltrs-raster
cargo publish -p pltrs-platform
cargo publish -p pltrs-text
cargo publish -p pltrs-render
cargo publish -p pltrs-ui
cargo publish -p pltrs
git switch main
```

Each command uploads one crate and waits until the crates.io index lists
it.

* If cargo reports a timeout, wait one minute. Then run the next
  command.
* If crates.io answers `429 Too Many Requests`, it published nothing of
  that crate. Wait for the time in the message, then run the same
  command again.

### 7. Publish the GitHub release (PUBLISHES)

Use the changelog section of the version as the release notes:

```sh
awk '/^## .*0\.1\.1/{on=1; next} /^## /{on=0} on' CHANGELOG.md > target/release-notes.md
gh release edit v0.1.1 --notes-file target/release-notes.md --draft=false
```

## Check the release

1. Make sure that https://pypi.org/project/pltrs/ shows the new version
   with the source distribution and six wheels.
2. Make sure that https://crates.io/crates/pltrs and the eight other
   crates show the new version.
3. Make sure that https://docs.rs/pltrs shows the documentation. docs.rs
   builds it some minutes after the upload.
4. Install pltrs (Python) in a new virtual environment and draw a
   figure:

   ```sh
   python -m pip install pltrs
   MPLBACKEND=agg python -c "import pltrs.pyplot as plt; plt.plot([1, 3, 2]); plt.savefig('check.png')"
   ```

5. Add pltrs (Rust) to a new crate and build it:

   ```sh
   cargo new check && cd check && cargo add pltrs@0.1.1 && cargo build
   ```

## If a step fails

* If a workflow job fails, nothing goes to PyPI. Correct the problem on
  `main`. Delete the tag with
  `git push --delete origin v0.1.1 && git tag -d v0.1.1`. Then do step 4
  again.
* If PyPI has the version but a crate did not publish, correct the
  crate. Then release the next version of all nine crates and of the
  Python package.
* If a release is defective, yank it. On PyPI, use the *Manage* page of
  the project. On crates.io, run `cargo yank --version 0.1.1 <crate>`
  for each crate. A yanked version stays available to projects that
  already use it.

## Setup for a new repository or account

These steps are complete for the pltrs repository. You need them again
only for a new repository or a new account.

* PyPI: add a trusted publisher for the project `pltrs` at
  https://pypi.org/manage/account/publishing/. Use the owner
  `Lorisk1210`, the repository `plt-rs`, the workflow `wheels.yml` and
  the environment `pypi`. GitHub stores no PyPI token.
* crates.io: create an API token with the scopes `publish-new` and
  `publish-update` at https://crates.io/settings/tokens. Store it with
  `cargo login` on the machine that publishes. Do not put the token in
  the repository.
* GitHub environment: in the repository settings, create the environment
  `pypi`. Add yourself as a required reviewer. Limit the deployment tags
  to `v*`.
* GitHub runners: the workflow tests the aarch64 Linux wheels on
  `ubuntu-24.04-arm`. If your plan does not have this runner, set the
  repository variable `LINUX_ARM64_RUNNER` to an arm64 Linux runner.
* `win_arm64`: to build this wheel in tag runs, set the repository
  variable `WIN_ARM64` to `true`. Set `WINDOWS_ARM64_RUNNER` if your
  plan does not have `windows-11-arm`. In a manual run, use the input
  `win_arm64` as an alternative.

## License files in the packages

cargo packages each crate from its directory. Thus each published crate
has copies of `LICENSE-MIT` and `LICENSE-APACHE`, and the applicable
parts of `THIRD_PARTY.md` and `LICENSES/`.

After you change `THIRD_PARTY.md` or `LICENSES/`, run
`python python/tools/crate_notices.py`. The script writes the copies.
The tests `python/tests/test_crates_io.py` and
`python/tests/test_build_backend.py` make sure that the copies agree.

## Build and check the archives on your machine

These commands publish nothing.

### Linux

1. Install Zig and the aarch64 Rust targets one time. Zig links the
   wheels against glibc 2.17 and musl.

   ```sh
   python/tools/zig.sh install
   rustup target add aarch64-unknown-linux-gnu aarch64-unknown-linux-musl
   ```

2. Build the source distribution, the alias wheel and the four Linux
   wheels into `target/wheels/`:

   ```sh
   cargo xtask python wheels
   ```

3. Install the archives into new virtual environments and run the tests:

   ```sh
   python python/tools/check_dist.py --install --python python3.12 \
       target/wheels/pltrs-*manylinux*x86_64.whl target/wheels/matplotlib-*.whl
   python python/tools/check_dist.py --install --lowest --python python3.12 \
       target/wheels/pltrs-*manylinux*x86_64.whl
   python python/tools/check_dist.py --install target/wheels/pltrs-*.tar.gz
   ```

Each wheel build takes about 10 minutes and about 6 GB of memory. Each
check takes about 10 minutes. `--lowest` installs the lowest permitted
versions of the dependencies. To run two checks at the same time, give
each check a different `--work` directory.

### macOS

1. Run `cargo xtask python wheels`. It builds the `arm64`, `x86_64` and
   `universal2` wheels, the source distribution and the alias wheel.
2. Check the `arm64` and `universal2` wheels with an arm64 Python:

   ```sh
   python python/tools/check_dist.py --install --python <arm64 python> \
       target/wheels/pltrs-*universal2.whl target/wheels/matplotlib-*.whl
   ```

3. Do not run an x86_64 Python with Rosetta to check the x86_64 code. A
   stack overflow in such a process can make Rosetta stop for all
   x86_64 programs. Examine the library in the unpacked wheel as an
   alternative:

   ```sh
   lipo -archs pltrs/_native.abi3.so            # x86_64 arm64
   otool -L pltrs/_native.abi3.so               # system libraries only
   nm -gU pltrs/_native.abi3.so | grep PyInit   # the entry point of the module
   ```

   The workflow job `macos-intel` runs the x86_64 code on an Intel Mac.

The check of the source distribution needs more than 4 GB of memory.

### Windows

1. Install the C++ build tools of Visual Studio.
2. If the default Rust host is `x86_64-pc-windows-gnu`, set
   `RUSTUP_TOOLCHAIN=1.99-x86_64-pc-windows-msvc`. The workflow links
   the wheel with MSVC.
3. Run `cargo xtask python wheels`. It builds the `win_amd64` wheel, the
   source distribution and the alias wheel.
4. Check the wheel. Give the full path of the interpreter, because each
   Python version has the name `python.exe`:

   ```sh
   python python/tools/check_dist.py --install --python C:/.../3.12/python.exe \
       target/wheels/pltrs-*win_amd64.whl target/wheels/matplotlib-*.whl
   ```

### The workflow file

After you change `.github/workflows/wheels.yml`, run
`python python/tools/check_workflows.py`.
