# nodmodcomp

`nodmodcomp` hibernates a project's `node_modules` directory into a compressed
archive, then restores it when the project is needed again.

The recommended workflow uses two high-level commands: `hibernate` puts an
inactive project into storage, and `run` temporarily restores its dependencies,
runs a command, and cleans them up again. The lower-level `pack` and `unpack`
commands remain available when manual control is needed.

It is intended for developers who keep many JavaScript projects locally and do
not want inactive `node_modules` directories consuming disk space.

## Why ?

JavaScript dependencies can occupy hundreds of megabytes or several gigabytes per project. When many projects are checked out at once, their
`node_modules` directories can consume a large part of a development machine's
storage even though most of those projects are inactive.

Deleting `node_modules` saves space, but restoring it means running the package
manager again. That can take time, require network access, and may not recreate
the exact installed tree if dependency metadata has changed.

`nodmodcomp` provides a middle ground: keep the installed dependency tree in a
compressed local archive and restore it when the project becomes active again.
It works independently of npm, pnpm, Yarn, or Bun; it is not a replacement for
those package managers.

## How it works

Packing a project changes this:

```text
my-project/
├── package.json
└── node_modules/
```

into this:

```text
my-project/
├── package.json
├── node_modules.pack
└── node_modules.pack.meta.json
```

- `node_modules.pack` is a Zstandard-compressed TAR archive.
- `node_modules.pack.meta.json` contains a snapshot of `package.json`.

During unpacking, the saved package snapshot is compared with the current
`package.json`. If dependencies or other top-level fields have changed,
`nodmodcomp` warns about the drift before continuing. The sidecar is advisory:
it never replaces or edits the project's `package.json`.

Symlinks are archived as symlinks instead of being followed. This is important
for pnpm dependency trees, including optional dependencies represented by
broken symlinks.

## Installation

### From source

Install a stable Rust toolchain, clone this repository, and run the following
from the repository root:

```sh
cargo run --release -- setup
```

This builds `nodmodcomp` and installs the running binary at:

```text
~/.local/bin/nodmodcomp
```

Make sure `~/.local/bin` is in `PATH`, then verify the installation:

```sh
nodmodcomp --version
```

To replace an existing installation with a newly built version:

```sh
cargo run --release -- setup --force
```

### From a release binary

The GitHub binary workflow produces archives for:

- Linux x86_64
- Linux ARM64
- macOS Intel
- macOS Apple Silicon

Download the archive matching your system from the repository's Releases page,
verify its adjacent `.sha256` file, extract it, and run:

```sh
./nodmodcomp setup
```

The macOS binaries are currently unsigned and not notarized, so macOS may ask
you to approve the binary before its first run.

## Quick start

### Hibernate a project

Use the high-level `hibernate` command when you are finished working on a
project or you want to archive an old one. Pass either the project path or `.` for the current directory:

```sh
cd my-project
nodmodcomp hibernate .
```

After the archive and metadata sidecar have been written successfully,
`node_modules` is removed.

You can also hibernate a project without changing directories:

```sh
nodmodcomp hibernate /path/to/my-project
```

Preview the operation without changing the project:

```sh
nodmodcomp hibernate --dry-run /path/to/my-project
```

### Resume work with automatic cleanup

Use the high-level `run` command instead of manually unpacking the project:

```sh
cd my-project
nodmodcomp run -- npm run dev
```

If `node_modules` is absent and `node_modules.pack` exists, `nodmodcomp`
unpacks it first. It then runs the requested command with normal terminal input,
waits for it to exit, and automatically packs `node_modules` again. This removes
the restored directory and leaves the project hibernated after the command
finishes, including when the command returns a non-zero exit status.

`run` tracks whether it performed the restore. If `node_modules` was already
unpacked, it runs the command directly, prints a skip message, and leaves the
directory unpacked instead of unexpectedly cleaning up user-managed state. The
command's exit status is preserved in both cases.

If no archive exists, the command is still executed. Any missing-dependency
error therefore comes from the requested command or package manager, and `run`
does not attempt to pack afterward because it did not restore anything.

## Commands

`hibernate` and `run` are the recommended high-level orchestrators. They use
the same packing and unpacking operations exposed by the individual `pack` and
`unpack` commands.

### `nodmodcomp hibernate [--dry-run] <path>`

High-level entry point for putting an inactive project into compressed local
storage. It validates the project path and `package.json`, then performs the
same safe packing operation as `pack`.

Use `--dry-run` to inspect what would be packed without modifying the project.

### `nodmodcomp run -- <command> [arguments...]`

High-level entry point for working with a hibernated project. It orchestrates
the complete lifecycle:

1. Restore `node_modules` when a packed archive is present.
2. Run the supplied command and wait for it to exit.
3. Repack and remove `node_modules` only when `run` performed the restore.
4. Return the supplied command's exit status.

The `--` clearly separates `nodmodcomp` arguments from the child command and is
recommended.

Examples:

```sh
nodmodcomp run -- npm test
nodmodcomp run -- pnpm dev
nodmodcomp run -- yarn build
nodmodcomp run -- npx vite
```

### `nodmodcomp pack`

Lower-level command for manually compressing the current project's
`node_modules`. It writes the package metadata sidecar and removes the original
directory after both outputs are safely in place.

The command refuses to overwrite an existing archive, sidecar, or temporary
output. Prefer `hibernate <path>` for the normal project-hibernation workflow.

### `nodmodcomp unpack`

Lower-level command for manually restoring `node_modules.pack` in the current
directory. It warns if the current `package.json` differs from the snapshot,
but continues after the warning.

It also continues with a warning if the sidecar is missing or unreadable. This
supports archives made before sidecar metadata was introduced.

The command refuses to unpack over an existing `node_modules` directory.
Use `run -- <command>` for the normal restore-run-cleanup workflow; use
`unpack` when you intentionally want the dependency directory to remain
available afterward.

### `nodmodcomp setup [--force]`

Installs the currently running executable as
`~/.local/bin/nodmodcomp`. Existing installations are preserved unless
`--force` is supplied.

## Package drift warnings

The metadata sidecar stores the parsed JSON value of `package.json`. Comparison
is semantic, so formatting changes and object key ordering do not produce false
warnings.

If top-level fields changed after packing, unpack reports their names. The
archive is still restored because the user may want to inspect it or run the
package manager afterward.

At present, `nodmodcomp` snapshots only `package.json`; it does not snapshot or
compare package-manager lockfiles.

## Safety behavior

- Archive and sidecar files are first written under temporary names.
- The original `node_modules` is removed only after both final files exist.
- Temporary outputs are cleaned up when packing fails.
- Existing archives, sidecars, temporary outputs, and dependency directories
  are not silently overwritten.
- The archive is removed only after extraction succeeds.
- pnpm-style symlinks are preserved rather than dereferenced.

## Recommended `.gitignore`

Archives represent local installed dependencies and can be large. They normally
should not be committed:

```gitignore
/node_modules.pack
/node_modules.pack.meta.json
/node_modules.temp.pack
/node_modules.temp.pack.meta.json
```

## Limitations

- This is local cold storage, not a transparent filesystem listener. Programs
  cannot trigger restoration merely by accessing `node_modules`; use `unpack`
  or launch them through `nodmodcomp run`.
- Restoring requires enough free space for the unpacked dependency tree while
  the compressed archive still exists.
- Packing temporarily requires space for both `node_modules` and the archive.
- Installed trees containing native binaries are tied to their operating
  system, CPU architecture, and sometimes system libraries. Do not assume an
  archive created on one machine will work on another.
- The archive captures installed files, not package-manager cache state.
- `run` automatically re-hibernates dependencies only when it restored them.
  If `node_modules` was already unpacked, `run` leaves it unchanged.
- The project is currently early-stage software.

## Development

Run the local quality checks with:

```sh
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo build --locked --release
```

GitHub Actions runs formatting, linting, and tests on Linux and macOS. The
binary workflow can be started manually, and tags matching `v*` create a GitHub
release with platform archives and SHA-256 checksums.

For example:

```sh
git tag v0.1.0
git push origin v0.1.0
```
