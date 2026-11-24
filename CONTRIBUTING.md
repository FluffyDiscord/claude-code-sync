# Contributing

Build, test, commit and release claude-code-sync. **Work goes straight to `master`. There are no pull requests.**

## Setup

1. Clone the repository.

   ```bash
   git clone https://github.com/FluffyDiscord/claude-code-sync.git
   cd claude-code-sync
   ```

2. Install the toolchain from `mise.toml`: Rust 1.96 with rustfmt and clippy, cargo-binstall and cargo-deny.

   ```bash
   mise install
   ```

3. Set a git identity. The integration tests create commits and fail without one.

   ```bash
   git config --global user.name "Your Name"
   git config --global user.email "you@example.com"
   ```

4. Check that everything passes.

   ```bash
   mise run ci
   ```

## Tasks

Every task lives in `mise.toml`, and CI runs the same ones. `mise run ci` runs `fmt-check`, `lint`, `build` and `test`.

| Task | Runs |
|---|---|
| `mise run fmt` | `cargo fmt --all` |
| `mise run fmt-check` | `cargo fmt --all -- --check` |
| `mise run lint` | `cargo clippy --all-targets --all-features -- -D warnings` |
| `mise run build` | `cargo build --verbose` |
| `mise run build-release -- --target <triple>` | `cargo build --release --verbose` for that target |
| `mise run test` | `cargo test --verbose -- --test-threads=1` |
| `mise run test-unit` | `cargo test --lib --verbose -- --test-threads=1` |
| `mise run test-doc` | `cargo test --doc --verbose` |
| `mise run test-integration` | `cargo test --test '*' --verbose -- --test-threads=1` |
| `mise run audit` | `cargo deny check advisories bans` |
| `mise run e2e` | A release build, then `--version` and `--help` on the binary |

Clippy warnings fail the build.

## Tests

```bash
cargo test -- --test-threads=1
```

**Always run the tests single-threaded.**
Every test shares one configuration directory, resolved by `ConfigManager` and overridable with `CLAUDE_CODE_SYNC_CONFIG_DIR`.
Parallel tests race on it. On macOS a parallel run can fall back to your real `~/Library/Application Support/claude-code-sync` and overwrite it.
`mise run test`, `test-unit` and `test-integration` pass `--test-threads=1` for you, and CI runs `mise run test`.

Integration tests live in `tests/`, and unit tests sit next to the code in `#[cfg(test)] mod tests`.
Tests that change process-wide state are marked `#[serial]` from `serial_test`.

`mise run test-integration` doesn't work on Windows.

## Dependency audit

```bash
mise run audit
```

`deny.toml` fails the audit on any of these:

- A crate with a security advisory, an unmaintained crate, an unsound crate or a yanked version.
- A crate on the ban list:

| Crate | Reason | Use instead |
|---|---|---|
| `atty` | It's unmaintained and unsound. | `std::io::IsTerminal` |
| `fuzzy-matcher` | It's archived upstream. | Nothing; `inquire` runs without its `fuzzy` feature. |
| `winapi` | It's abandoned. | `windows-sys` |

`winapi` is allowed only through `crossterm` and `crossterm_winapi`, which need it on Windows.
Several versions of one crate are allowed.

## Dependency updates

Nothing to do. Renovate (`renovate.json`) handles them:

- It runs after 10 pm and before 5 am on weekdays, and all weekend.
- Every update, major ones included, merges on its own once the release is 3 days old and CI passes on its `renovate/**` branch.
- Security fixes merge on their own too.
- It commits as `deps: …` and bumps the version range in `Cargo.toml`.
- A pull request opens only after you approve it on the dependency dashboard.

## CI

| Workflow | Runs on | Does |
|---|---|---|
| `ci.yml` | A push to `master` or `renovate/**`, and manual runs | Builds and tests on Ubuntu, macOS and Windows. Checks formatting and clippy, smoke-tests a release build and runs the dependency audit. |
| `build.yml` | A push to `master` that changes `Cargo.toml`, `Cargo.lock`, `mise.toml`, `src/self_update.rs`, `build.yml` or `release.yml`. `release.yml` also calls it. | Builds the release binaries for all 8 targets and runs `--version` on each. On a release it uploads them. |
| `release.yml` | A pushed `v*` tag | Creates the GitHub release and calls `build.yml`. |

The `ci.yml` and `build.yml` jobs install their tools with `jdx/mise-action` and run the mise tasks above.

`build.yml` builds these targets, each on its own native runner:

- `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`, `aarch64-unknown-linux-musl`
- `x86_64-apple-darwin`, `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`

**Don't rename the release assets.** `self-update` and the README's download links build the asset names themselves, for example `claude-code-sync-linux-x86_64-musl.tar.gz` and `claude-code-sync-windows-x86_64.exe.zip`.
Each archive ships with a `.sha256` file.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/): a type, an optional scope, then a short description.

```text
feat(push): push every transcript and move mapped projects under their id
fix(cache): keep transcript summaries across releases
refactor!: <a breaking change>
```

- The types in use are `feat`, `fix`, `perf`, `refactor`, `test`, `ci`, `docs` and `chore`.
- `!` after the type marks a breaking change.
- A release commit is `chore: release X.Y.Z`.

Every commit subject since the previous tag becomes a line in the release notes, so write it for the reader of the release.

There's no `CHANGELOG.md`; the GitHub release notes are the changelog.

## Releasing

**A pushed `v*` tag is a release.** Bump the version by [Semantic Versioning](https://semver.org/).

1. Set `version` in `Cargo.toml`, then update `Cargo.lock` to match.

   ```bash
   cargo check
   ```

2. Commit both files.

   ```bash
   git commit -am "chore: release X.Y.Z"
   ```

3. Tag the commit.

   ```bash
   git tag vX.Y.Z
   ```

4. Push `master` and the tag together.

   ```bash
   git push origin master vX.Y.Z
   ```

5. Wait for `release.yml` to finish. It runs these steps:
   1. It checks that the `Cargo.toml` version matches the tag. A mismatch fails the run before anything is built.
   2. It creates the GitHub release. The notes list every commit since the previous tag, merges left out.
   3. `build.yml` builds the 8 targets and attaches each archive with its checksum.

6. Update your installed binary.

   ```bash
   claude-code-sync self-update
   ```

   `sync` also updates itself to the latest release before it runs. Set `CLAUDE_CODE_SYNC_NO_UPDATE=1` to skip that.
   A binary installed with `cargo install` into `~/.cargo/bin` refuses to update itself. Pass `--force` to update it anyway.

## Local cross-builds

```bash
bin/release.sh
```

It builds Linux (musl), Windows and a universal macOS binary into `dist/` from one Linux or macOS host.

- It needs the rustup targets the script lists, `cargo-zigbuild` (`cargo install cargo-zigbuild`), zig and python3.
- It uses the `dist` profile from `Cargo.toml`, which is built for size. The `release` profile, used by `cargo install` and by CI, stays as the crate sets it.
- Published releases always come from CI, not from this script.

## Code

- Return `anyhow::Result` and add context to errors, for example `.with_context(|| format!("Failed to read {}", path.display()))`.
- Update `README.md` for user-facing changes and `docs/architecture.md` for changes to the internals.
- Track open work in `TODO.md`.

## Bug reports

Open an issue with these details:

- The steps and commands that reproduce it.
- What you expected and what happened.
- Your OS and the output of `claude-code-sync --version`.
- The error message or log.

## License

Contributions are licensed under the MIT License.
