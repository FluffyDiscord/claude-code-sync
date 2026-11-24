# claude-code-sync

[![CI](https://github.com/FluffyDiscord/claude-code-sync/actions/workflows/ci.yml/badge.svg)](https://github.com/FluffyDiscord/claude-code-sync/actions/workflows/ci.yml)
[![Release](https://github.com/FluffyDiscord/claude-code-sync/actions/workflows/release.yml/badge.svg)](https://github.com/FluffyDiscord/claude-code-sync/actions/workflows/release.yml)

Sync Claude Code conversations, settings, skills and rules between machines through a git repository you own.

New here: follow [Usage](#usage). Second machine: see [Add another machine](#add-another-machine).

![Demo](demo1.svg)

## Features

- [Conversation sync](#how-a-pull-merges) — the same session continued on two machines merges into one transcript
- [Artifact sync](#artifact-sync) — settings, memory, skills, agents, commands, rules, hooks, plugin manifests, plans, todos and prompt history follow you
- [Project map](#project-map) — a project kept at a different path on each machine stays one project; unmapped projects never download
- [Machine-neutral paths](#machine-neutral-paths) — hook commands and status lines that name a path work on every machine
- [Deletions that propagate](#deletions-that-propagate) — delete a skill once, it's gone everywhere; a fresh machine can't wipe the repo
- [External merge tool](#external-merge-tool) — resolve a differing file in a three-way merge window instead of picking a side
- [Purge](#purge) — drop old transcripts from the machine and the repo together, never sooner than Claude Code would
- [Secrets guard](#what-is-never-synced) — credentials, local settings and keys never leave the machine, whatever the config says
- [Undo](#undo) — reverse the last pull or push from an automatic snapshot
- [Self-update](#self-update) — `sync` installs the latest release before it runs

## Install

Download the build for your platform. Linux x86_64:

```bash
mkdir -p ~/.local/bin
curl -fsSL https://github.com/FluffyDiscord/claude-code-sync/releases/latest/download/claude-code-sync-linux-x86_64-musl.tar.gz \
  | tar -xz -C ~/.local/bin
```

`~/.local/bin` must be on your `PATH`. **To update:** `claude-code-sync self-update`. `sync` also updates itself.

Every [release](https://github.com/FluffyDiscord/claude-code-sync/releases) has these assets, each with a `.sha256` file:

| Platform | Asset |
|---|---|
| Linux x86_64, static | `claude-code-sync-linux-x86_64-musl.tar.gz` |
| Linux x86_64, glibc | `claude-code-sync-linux-x86_64.tar.gz` |
| Linux ARM64, static | `claude-code-sync-linux-aarch64-musl.tar.gz` |
| Linux ARM64, glibc | `claude-code-sync-linux-aarch64.tar.gz` |
| macOS Apple Silicon | `claude-code-sync-macos-aarch64.tar.gz` |
| macOS Intel | `claude-code-sync-macos-x86_64.tar.gz` |
| Windows x86_64 | `claude-code-sync-windows-x86_64.exe.zip` |
| Windows ARM64 | `claude-code-sync-windows-aarch64.exe.zip` |

The static musl builds have no dependencies and run on any distro, Alpine and NixOS included.

### Cargo

```bash
cargo install --locked --git https://github.com/FluffyDiscord/claude-code-sync
```

To update, run the same command again. `self-update` and `sync` leave a binary in `~/.cargo/bin` alone and print this command instead.

> The `claude-code-sync` crate on crates.io is the original upstream project, not this fork.

You also need `git` on your `PATH`. Git LFS is optional; see [Git LFS](#git-lfs).

## Usage

1. Create an empty **private** repository on GitHub, GitLab or any git host, and check that git can reach it:

   ```bash
   ssh -T git@github.com
   ```

   No access yet → see [Remote repository access](#remote-repository-access).

2. Run the setup wizard:

   ```bash
   claude-code-sync init
   ```

   It asks for the remote URL (or a local directory), where to clone it, whether to skip attachments, how old a conversation may be, and which [artifact categories](#artifact-sync) to sync. Running any other command first starts the same wizard.

3. Map each project you want to sync on this machine:

   ```bash
   claude-code-sync config --map-project /home/user/work/app
   ```

   Only mapped projects are pulled. Push sends every transcript anyway. See [Project map](#project-map).

4. Sync:

   ```bash
   claude-code-sync sync
   ```

   `sync` pulls, merges, then pushes. `claude-code-sync` with no command does the same.

### Add another machine

1. Install, then clone the same repository:

   ```bash
   claude-code-sync init --remote git@github.com:you/claude-history.git
   ```

   The clone goes to the [config directory](#files-and-directories)`/repo`. Add `--clone --local <path>` to put it elsewhere.

2. Turn on the artifact categories you want here (each machine keeps its own list):

   ```bash
   claude-code-sync config --enable-artifacts all
   ```

3. Map the projects this machine works on, under the same ids as on the first machine:

   ```bash
   claude-code-sync config --map-project app=/Users/you/src/app
   ```

4. Sync:

   ```bash
   claude-code-sync sync
   ```

Settings, skills, agents, commands, rules, plugin manifests and prompt history land in `~/.claude`.

### Non-interactive setup

For scripts and fresh machines, put the answers in a file:

`~/.claude-code-sync-init.toml`

```toml
repo_path = "~/claude-history-sync"
remote_url = "git@github.com:you/claude-history.git"
clone = true
exclude_attachments = false
# exclude_older_than_days = 30
enable_lfs = false
sync_subdirectory = "projects"
use_project_name_only = false

[sync_artifacts]
settings = true
memory = true
skills = true
agents = true
commands = true
rules = true
plugins = true
prompt_history = true
```

```bash
claude-code-sync init
```

`init` with no flags, or any first command, reads the first file it finds:

1. `$CLAUDE_CODE_SYNC_INIT_CONFIG`
2. `~/.claude-code-sync-init.toml`
3. `<config dir>/init.toml`

Pass a file directly with `init --config <path>`. `clone = true` needs `remote_url`.

`config --export` writes the current setup as `.claude-code-sync-init.toml` in the current directory. Copy it to `~` on the new machine and run `init`. Include and exclude patterns, the project map and the merge tool aren't exported.

## How a pull merges

Claude Code keeps each conversation as a `.jsonl` file under `~/.claude/projects/<project>/`. A pull compares this machine's copy with the repo's.

| Situation | What happens |
|---|---|
| Session only exists on one side | Copied to the other side |
| Same session changed on both machines | Merged message by message: every message from both copies is kept, and an edited message keeps its newer version |
| Merge fails (damaged file) in a terminal | You see why and pick `local`, `remote`, `both (remote saved as a copy)` or `details` |
| Merge fails with no terminal | Local copy stays; the remote one is saved as `<session>-conflict-<YYYYMMDD-HHMMSS>.jsonl` next to it |

A remote version that's already saved as a `-conflict-` copy isn't saved again.

Every conflict goes into a report: `claude-code-sync report`. Merge internals: [docs/architecture.md](docs/architecture.md).

**Git-level conflicts.** When both machines pushed, the pull merges the two branches itself.

- A file both changed only in date-times (`lastUpdated`, `modified: …`) keeps the later dates, no prompt.
- Any other file both changed asks, per file: `remote`, `local`, `merge` (with a [merge tool](#external-merge-tool)) or `stop`.
- `stop`, no terminal to ask in, or an unreachable remote → the merge is undone and the pull stops.

## Artifact sync

Each machine picks which parts of `~/.claude` to sync besides conversations. All categories start off; the setup wizard pre-selects all but `hooks`.

| Category | Covers | Notes |
|---|---|---|
| `settings` | `settings.json`, `keybindings.json` | Paths made [machine-neutral](#machine-neutral-paths) |
| `memory` | `~/.claude/CLAUDE.md` | Global user memory |
| `skills` | `~/.claude/skills/` | Whole tree |
| `agents` | `~/.claude/agents/` | Subagent definitions |
| `commands` | `~/.claude/commands/` | Slash commands |
| `rules` | `~/.claude/rules/` | Rule files |
| `hooks` | `~/.claude/hooks/` | Scripts that run automatically; see below |
| `plugins` | `plugins/installed_plugins.json`, `plugins/known_marketplaces.json` | Manifests only, never plugin caches |
| `plans` | `~/.claude/plans/` | Plan-mode documents; may hold sensitive text |
| `todos` | `~/.claude/todos/` | Session task lists; change often |
| `prompt-history` | `~/.claude/history.jsonl` | Lines merged from both sides, never overwritten |

```bash
claude-code-sync config --enable-artifacts all
claude-code-sync config --enable-artifacts settings,skills,agents,commands,plugins,prompt-history
claude-code-sync config --disable-artifacts todos
```

`all` turns on everything except `hooks`. `status` shows which categories differ from the repo.

**Attachments** aren't a category. Images, PDFs and each project's `memory/` folder under `~/.claude/projects/` sync unless you set `--exclude-attachments`.

A `.gitignore` inside a synced folder (say `~/.claude/skills/x/.gitignore`) keeps the files it matches out of the repo.

### Hooks run code

**Enable `hooks` only for a repository you control.** Everything in `~/.claude/hooks/` runs automatically, and the `settings.json` that registers it syncs too. It's only turned on by name:

```bash
claude-code-sync config --enable-artifacts hooks
```

An interactive pull asks before it makes a file executable. A file the repository adds is still written without asking.

### The executable bit

- Only the executable bit travels; a `chmod +x` with no content change still syncs.
- It's only ever granted, never removed. Repeat a `chmod -x` on each machine.
- A pull never widens a local file's mode, and a file a pull creates starts private.
- It needs `core.fileMode = true` in the sync repo, git's default everywhere but Windows checkouts.

### What is never synced

A built-in denylist applies in both directions and no setting overrides it:

- Files: `.credentials.json`, `settings.local.json`, `.claude.json`, `stats-cache.json`, `.last-update-result.json`, `mcp-needs-auth-cache.json`, `*.pem`, `*.key`, `.env*`, `daemon*`
- Folders: `shell-snapshots/`, `session-env/`, `file-history/`, `paste-cache/`, `cache/`, `debug/`, `statsig/`, `backups/`, `sessions/`, `.trash/`

A pull refuses these paths even when the repo holds them. Every push also keeps a guard block in the repo's `.gitignore`.

### Conflict policy

**A pull takes only what the other machine changed.** Each machine remembers the version it last synced and compares all three.

| Since the last sync | Pull does |
|---|---|
| Only the remote changed it | Takes the remote version |
| Only this machine changed it | Keeps it; the next push sends it |
| This machine deleted it (skills, agents, commands, rules, hooks) | Leaves it deleted; the next push removes it. Edited remotely → restores it |
| The remote deleted it, this machine edited it | Keeps it and warns; the next push restores it |
| Both changed only its dates | Keeps the later dates |
| Both changed it | Asks `remote` / `local` (plus `merge` with a [merge tool](#external-merge-tool)) in a terminal; otherwise keeps this machine's version and warns |
| Never synced on this machine | Takes the remote version |

- Every file a pull overwrites or deletes is snapshotted first, and files it creates are recorded, so `undo pull` can put them back.
- `pull --interactive` confirms each overwrite and each deletion.
- `history.jsonl` is never overwritten: push and pull both merge the union of lines, so prompt history only grows. LFS-tracked copies still merge.

### Memory indexes

A project's `memory/MEMORY.md` merges by link target in both directions, like prompt history.

- Your file keeps its headings, blank lines, prose and entry order.
- Entries only the other side has are appended.
- Only entry lines travel. A heading or note reaches the other machine only if your file created its copy.

### Sync repository layout

```
<sync repo>/
  .gitignore              # managed never-sync guard block
  projects/               # transcripts and attachments (sync_subdirectory)
  artifacts/
    settings/  memory/  skills/  agents/  commands/  rules/
    hooks/     plugins/ plans/   todos/   prompt-history/
```

## Machines that differ

### Project map

Claude Code names a project folder after its absolute path, so one project is `-home-user-work-app` on Linux and `-Users-you-src-app-renamed` on a Mac. Map it once per machine and both store it as `projects/app/`:

```bash
# Linux: the id defaults to the folder name, `app`
claude-code-sync config --map-project /home/user/work/app

# Mac, where it lives elsewhere under another name
claude-code-sync config --map-project app=/Users/you/src/app-renamed

# Drop a mapping
claude-code-sync config --unmap-project app
```

Mapping a project syncs it right away. Each machine writes it back into its own folder in `~/.claude/projects`, which keeps its name because Claude Code reads only that one.

| Rule | |
|---|---|
| Id | A plain folder name |
| Path | Absolute. `--map-project` drops a trailing slash; in `config.toml` the path must be written without one |
| Id already taken by another path | Refused. Use another id: `--map-project personal-app=/home/user/personal/app` |
| Two ids encoding to one folder | Refused |
| Move an id to a new path | `--unmap-project app --map-project app=/new/path` |
| Repo already holds the project under its encoded path | The next push moves it into `projects/app/`; a file already in `projects/app/` wins |

**Only mapped projects pull.** This machine's clone downloads only mapped projects, the folders of its own projects, and the shared artifacts. Other machines' projects never land here, so `purge` here prunes only those.

**Push sends every transcript of this machine.** Unmapped projects go to the repo under their encoded path as a backup. Their project memory and attachments stay here.

- Push skips a folder named after another machine's home (`-home-k-…` on a Mac, `C--Users-Admin-…` on Linux) unless it's mapped.
- A repo copy with later messages than the local one is never replaced.

**To sync every project instead:**

```bash
claude-code-sync config --mapped-projects-only false
```

- The whole repo downloads.
- Unmapped projects use their encoded path, or the folder name with `--use-project-name-only true`.
- A file under an id this machine hasn't mapped is skipped, not written where Claude Code would never read it. Run `--map-project` here.
- All skipped files come in one warning per pull. One line per file instead: `config --warn-each-skipped-file true`.

### Machine-neutral paths

`settings` and `plugins` are stored with this machine's paths replaced by `__CLAUDE_DIR__` and `__HOME__`, and written back with the local paths on pull. Any value that names a path travels: hook commands, the status line, new keys too. Only text files are rewritten.

### Deletions that propagate

Delete a file in `skills`, `agents`, `commands`, `rules`, `hooks` or a project's `memory/` folder, and the next push removes it from the repo; the other machines' next pull removes it there. Transcripts, other attachments, plans, todos and prompt history never propagate deletions.

| Guard | Effect |
|---|---|
| Machine has never synced | Deletes nothing, so a fresh clone can't wipe the repo |
| Category folder missing locally | Skipped on push |
| Category missing from the repo | Skipped on pull, so an old branch doesn't read as "all deleted" |

- Deleting a category's last file still propagates.
- Deleted files are snapshotted: `undo pull` brings them back, and `pull --interactive` asks first.
- `undo push` resets this machine's record for the repo; the next sync rebuilds it.

How the record works: [docs/architecture.md](docs/architecture.md).

## Commands

Commands other than `init`, `config` and `self-update` start the setup wizard on first run. `push`, `pull`, `sync`, `undo` and `cleanup-snapshots` accept `-v, --verbose` and `-q, --quiet`.

### `init`

Sets up the sync repository. With no flags it reads an [init file](#non-interactive-setup), or runs the wizard.

```bash
claude-code-sync init --remote git@github.com:you/claude-history.git                    # clone into <config dir>/repo
claude-code-sync init --clone --local ~/claude-sync --remote git@github.com:you/claude-history.git
claude-code-sync init --local ~/claude-sync                                              # new or existing local repo
claude-code-sync init --local ~/claude-sync --remote git@github.com:you/claude-history.git  # new local repo, adds the remote
claude-code-sync init --config ./init.toml
```

| Option | Meaning |
|---|---|
| `-l, --local <PATH>` | Where the sync repo lives |
| `-r, --remote <URL>` | Remote URL: `https://`, `http://`, `git@` or `ssh://`. Alone → clones into `<config dir>/repo` |
| `--clone` | Clone `--remote` into `--local` |
| `-c, --config <PATH>` | Read an [init file](#non-interactive-setup) |

### `sync`

Pulls, runs [purge](#purge) if `purge_after_sync` is on, then pushes.

```bash
claude-code-sync sync
claude-code-sync sync -m "Daily sync" --exclude-attachments
```

| Option | Meaning |
|---|---|
| `-m, --message <MSG>` | Commit message |
| `-b, --branch <BRANCH>` | Branch; default is the current one |
| `--exclude-attachments` | Push `.jsonl` transcripts only |
| `-i, --interactive` | Preview and confirm |

**`sync` updates itself first.** A newer release → it downloads it, checks the SHA-256, replaces the binary and restarts with the same arguments. Offline, or the check fails → it warns and syncs on the current version. A `cargo install` binary is left alone and `sync` prints the cargo command.

To stay on the installed version:

```bash
CLAUDE_CODE_SYNC_NO_UPDATE=1 claude-code-sync sync
```

### `push`

Copies local history into the sync repo, commits, and pushes to `origin`. Every transcript is pushed, mapped project or not; project memory and attachments follow `mapped_projects_only`.

```bash
claude-code-sync push -m "Weekly backup"
claude-code-sync push --branch backup --exclude-attachments
```

| Option | Meaning |
|---|---|
| `-m, --message <MSG>` | Commit message |
| `-b, --branch <BRANCH>` | Branch; default is the current one |
| `--exclude-attachments` | Push `.jsonl` transcripts only |
| `--push-remote <BOOL>` | Push to `origin` after committing; default `true` |
| `-i, --interactive` | Show the plan and confirm; no → nothing is copied or committed |

**A failed push to the remote fails the command.** The commit stays in the local sync repo, and the next `push` or `sync` sends it, even with no new changes.

### `pull`

Fetches the remote and merges it into `~/.claude`. See [How a pull merges](#how-a-pull-merges) and [Conflict policy](#conflict-policy).

```bash
claude-code-sync pull
claude-code-sync pull --fetch-remote false
```

| Option | Default | Meaning |
|---|---|---|
| `--fetch-remote <BOOL>` | `true` | `false` → merge only what's already in the local sync repo |
| `-b, --branch <BRANCH>` | current | Branch to pull |
| `-i, --interactive` | off | Preview, and confirm each overwritten or deleted file |

> `pull --interactive` asks `Pull?` after it has already merged the remote into the sync repo. Answering no leaves `~/.claude` untouched, but the sync repo stays merged.

### `status`

Shows the repo path, remote, branch, uncommitted changes, session counts and per-category artifact drift.

```bash
claude-code-sync status --show-conflicts --show-files
```

| Option | Meaning |
|---|---|
| `--show-conflicts` | Conflicts from the last sync |
| `--show-files` | Local session files that would sync |

### `config`

Changes settings in [the config file](#configuration). With no flags it opens a menu to switch the active repository or edit filters.

```bash
claude-code-sync config --show
claude-code-sync config --interactive      # pick a setting to change
claude-code-sync config --wizard           # step through every setting
claude-code-sync config --exclude-projects "*test*,*temp*"
claude-code-sync config --enable-lfs true --lfs-patterns "*.jsonl,*.png"
```

| Option | Config key |
|---|---|
| `--exclude-older-than <DAYS>` | `exclude_older_than_days` |
| `--include-projects <PATTERNS>` | `include_patterns` |
| `--exclude-projects <PATTERNS>` | `exclude_patterns` |
| `--exclude-attachments <BOOL>` | `exclude_attachments` |
| `--enable-lfs <BOOL>` | `enable_lfs` |
| `--lfs-patterns <PATTERNS>` | `lfs_patterns` |
| `--sync-subdirectory <DIR>` | `sync_subdirectory` |
| `--use-project-name-only <BOOL>` | `use_project_name_only` |
| `--enable-artifacts <NAMES>` / `--disable-artifacts <NAMES>` | `[sync_artifacts]`; names or `all` |
| `--map-project [ID=]PATH` / `--unmap-project <ID>` | `[project_map]`; repeatable |
| `--mapped-projects-only <BOOL>` | `mapped_projects_only` |
| `--warn-each-skipped-file <BOOL>` | `warn_each_skipped_file` |
| `--merge-tool <CMD>` | `merge_tool`; `""` clears it |
| `--prefer-merge-tool <BOOL>` | `prefer_merge_tool` |
| `--purge-older-than <DAYS>` | `purge_older_than_days`; minimum 30 |
| `--purge-after-sync <BOOL>` | `purge_after_sync` |
| `--show` | Print the current config |
| `-i, --interactive` | Menu of settings |
| `-w, --wizard` | Step-by-step wizard |
| `--export` | Write `.claude-code-sync-init.toml` to the current directory |

Pattern lists are comma-separated.

### `purge`

Deletes transcripts past the retention window from this machine **and** the sync repo. Removing them on one side only would bring them back on the next sync.

```bash
claude-code-sync purge --dry-run          # what would go, and how much space
claude-code-sync purge                    # asks first
claude-code-sync purge --yes              # for scripts; required outside a terminal
claude-code-sync purge --older-than 365
```

| Option | Default | Meaning |
|---|---|---|
| `--older-than <DAYS>` | see below | Window for this run; minimum 30 |
| `--dry-run` | off | Show the plan and stop |
| `-y, --yes` | off | Delete without asking |

**The default window is the longer of 180 days and this machine's Claude Code retention** (`cleanupPeriodDays` in `settings.json` or `settings.local.json`, 30 days if unset). Claude Code deletes those transcripts itself, so the repo never keeps fewer. A window you set wins.

To purge on every sync, between the pull and the push:

```bash
claude-code-sync config --purge-older-than 365
claude-code-sync config --purge-after-sync true
```

- A transcript's age is its last message, not the file date: a conversation pulled today is still as old as it was.
- A transcript with no readable timestamp is never purged.
- Each removal takes the session's `subagents/` and `tool-results/` folders and any superseded or orphaned copies with it.
- `purge` commits the removal on its own and doesn't push it; the next `push` or `sync` sends it. With `purge_after_sync`, the removal lands in the push commit.
- Git history keeps the purged files until you rewrite it.
- The sync repo must have no uncommitted changes; run `push` first.

### `report`

Prints the conflict report from the last sync.

```bash
claude-code-sync report
claude-code-sync report --format json --output conflicts.json
```

| Option | Default | Meaning |
|---|---|---|
| `-f, --format <FORMAT>` | `markdown` | `json` or `markdown` (`md`). Any other value prints a short summary to the terminal |
| `-o, --output <FILE>` | stdout | Write to a file; `json` or `markdown` only |

### `remote`

Shows or changes the sync repo's git remote. Push and pull use `origin`.

```bash
claude-code-sync remote show
claude-code-sync remote set git@github.com:you/claude-history.git
claude-code-sync remote set --name origin https://github.com/you/claude-history.git
claude-code-sync remote remove
```

| Subcommand | Meaning |
|---|---|
| `show` | Sync directory, branch and each remote's URL |
| `set [-n NAME] <URL>` | Set or change a remote; `NAME` defaults to `origin`. URL starts with `https://`, `http://`, `git@` or `ssh://` |
| `remove [-n NAME]` | Remove a remote; `NAME` defaults to `origin` |

Setting or removing `origin` also updates the remote saved in `state.json`. With no `origin`, push only commits locally.

### `undo`

Reverses the last pull or push.

```bash
claude-code-sync undo pull --preview
claude-code-sync undo pull
claude-code-sync undo push
```

| Subcommand | Effect |
|---|---|
| `pull` | Restores every file the pull changed or deleted and removes the files it created: transcripts, `-conflict-` copies and artifacts |
| `push` | Moves the sync repo back to the commit before the push (`git reset --soft`); the pushed files stay staged |

> A file you've edited since the pull stays as it is, with a message. A pull that fails halfway is undone the same way.

- `--preview` shows what would be restored, deleted and kept, then stops.
- A successful undo deletes its snapshot and updates the history. A failed one keeps both, so you can run it again.
- `undo push` doesn't touch the remote. If the push reached it, run `git push --force origin <branch>` in the sync repo.

### `history`

Lists past pulls and pushes: type, time, branch, conversations affected, added / modified / conflict / unchanged counts, and whether undo is available. The last 5 operations are kept.

```bash
claude-code-sync history list
claude-code-sync history last -t pull
claude-code-sync history review
claude-code-sync history clear
```

| Subcommand | Meaning |
|---|---|
| `list [-l N]` | Recent operations; `N` defaults to 10 |
| `last [-t pull\|push]` | Details of the last operation |
| `review [-l N]` | Pick an operation to inspect; needs a terminal |
| `clear` | Delete the history |

### `cleanup-snapshots`

Deletes old undo snapshots. A snapshot goes only when it's both past the newest `--max-count` of its type and older than `--max-age-days`.

```bash
claude-code-sync cleanup-snapshots --dry-run
claude-code-sync cleanup-snapshots --max-count 3 --max-age-days 14
```

| Option | Default |
|---|---|
| `--max-count <N>` | `5` per operation type |
| `--max-age-days <DAYS>` | `7` |
| `--dry-run` | off |
| `-i, --interactive` | off |

### `self-update`

Downloads the release for your platform, checks its SHA-256 and replaces the binary.

```bash
claude-code-sync self-update --check      # report only
claude-code-sync self-update
claude-code-sync self-update --to v0.5.4  # any release, downgrades included
claude-code-sync self-update --yes        # for scripts; required outside a terminal
```

| Option | Meaning |
|---|---|
| `--check` | Report whether an update exists |
| `--to <VERSION>` | Install this release instead of the latest |
| `--force` | Reinstall the current version, or replace a `cargo install` binary |
| `-y, --yes` | Don't ask |

It shows the version change and asks first. A binary in `~/.cargo/bin` is refused with the cargo command, so cargo's records stay right; `--force` replaces it anyway.

## External merge tool

**Get `merge` next to `remote` / `local` in the per-file prompt.** It appears for a file both machines changed, and for a pulled file that differs under `pull --interactive`.

```bash
claude-code-sync config --merge-tool "phpstorm merge"
claude-code-sync config --prefer-merge-tool true   # prompts start on merge
```

The tool runs as `<merge_tool> <local> <remote> <base> <output>`, the JetBrains argument order. Whatever it saves to `<output>` is used, even the local version unchanged.

| Case | `<base>` |
|---|---|
| Both machines changed the file | The version both started from |
| Pulled file differs | Empty; nothing records an ancestor |

The prompt waits 15 minutes for the tool. Change it with `CLAUDE_CODE_SYNC_MERGE_TIMEOUT_SECONDS`. `--merge-tool ""` goes back to the terminal picker.

## Configuration

Settings live in `config.toml` in the [config directory](#files-and-directories). Defaults:

```toml
# exclude_older_than_days = 30   # unset: no age limit
include_patterns = []
exclude_patterns = []
max_file_size_bytes = 10485760
exclude_attachments = false
enable_lfs = false
lfs_patterns = ["*.jsonl"]
sync_subdirectory = "projects"
use_project_name_only = false
mapped_projects_only = true
# purge_older_than_days = 365    # unset: longer of 180 and cleanupPeriodDays
purge_after_sync = false
warn_each_skipped_file = false
merge_tool = ""
prefer_merge_tool = false

[sync_artifacts]
settings = false
memory = false
skills = false
agents = false
commands = false
rules = false
hooks = false
plugins = false
plans = false
todos = false
prompt_history = false

[project_map]
# app = "/home/user/work/app"
```

| Option | Default | Meaning |
|---|---|---|
| `exclude_older_than_days` | unset | Skip files not modified in this many days |
| `include_patterns` | `[]` | Sync only paths matching one; `*` wildcard, no `*` = substring |
| `exclude_patterns` | `[]` | Skip paths matching one |
| `max_file_size_bytes` | `10485760` (10 MB) | Skip larger files; `0` → warning, 10 MB used |
| `exclude_attachments` | `false` | `.jsonl` transcripts only |
| `enable_lfs` | `false` | Store `lfs_patterns` in [Git LFS](#git-lfs) |
| `lfs_patterns` | `["*.jsonl"]` | Files LFS tracks |
| `sync_subdirectory` | `"projects"` | Repo folder for transcripts; must stay inside the repo, `artifacts` is reserved |
| `use_project_name_only` | `false` | Unmapped projects stored under their folder name |
| `mapped_projects_only` | `true` | Pull only [mapped projects](#project-map) |
| `purge_older_than_days` | unset | [Purge](#purge) window; minimum 30 |
| `purge_after_sync` | `false` | Purge during `sync` |
| `warn_each_skipped_file` | `false` | One warning per unplaced file instead of one total |
| `merge_tool` | `""` | [Merge tool](#external-merge-tool) command |
| `prefer_merge_tool` | `false` | Prompts start on `merge` |
| `[sync_artifacts]` | all `false` | [Artifact categories](#artifact-sync) |
| `[project_map]` | empty | `id = "/absolute/path"` |

`max_file_size_bytes` has no flag. Set it in MB under `config --interactive`, or edit the file.

### Git LFS

```bash
claude-code-sync config --enable-lfs true
```

The next push runs `git lfs install --local` in the sync repo and writes the patterns to `.gitattributes`. Install `git-lfs` first (`brew install git-lfs`, `apt install git-lfs`, or [git-lfs.com](https://git-lfs.com) on Windows).

### Files and directories

| OS | Config directory |
|---|---|
| Linux | `$XDG_CONFIG_HOME/claude-code-sync`, or `~/.config/claude-code-sync` |
| macOS | `~/Library/Application Support/claude-code-sync` |
| Windows | `%APPDATA%\claude-code-sync` |
| Other | `~/.claude-code-sync` |

It holds:

| File | Content |
|---|---|
| `config.toml` | [Configuration](#configuration) |
| `state.json` | Sync repo path and remote |
| `repo/` | Default clone location |
| `operation-history.json` | Last 5 pulls and pushes |
| `snapshots/` | Undo snapshots |
| `latest-conflict-report.json` | Last conflict report |
| `claude-code-sync.log` | One startup line per run |
| `session-summaries.json` | Transcript summary cache |
| `init.toml` | Optional [init file](#non-interactive-setup) |

### Environment variables

| Variable | Effect |
|---|---|
| `CLAUDE_CODE_SYNC_NO_UPDATE` | Set → `sync` skips the update check |
| `CLAUDE_CODE_SYNC_INIT_CONFIG` | Path to an [init file](#non-interactive-setup) |
| `CLAUDE_CODE_SYNC_CONFIG_DIR` | Use `<value>/claude-code-sync` as the config directory |
| `CLAUDE_CODE_SYNC_CLAUDE_DIR` | Use this instead of `~/.claude` |
| `CLAUDE_CODE_SYNC_MERGE_TIMEOUT_SECONDS` | Merge tool wait; default 900 |
| `RUST_LOG` | Console log level; default `info` |

## Logging

Console output follows `RUST_LOG`: `trace`, `debug`, `info` (default), `warn`, `error` or `off`.

```bash
RUST_LOG=debug claude-code-sync sync
```

Log messages go to the console only. `claude-code-sync.log` in the [config directory](#files-and-directories) gets one startup line per run. Over 10 MB it's renamed to `claude-code-sync.log.old` on the next start.

## Scheduled sync

Cron, every night at 2:00:

```bash
0 2 * * * $HOME/.local/bin/claude-code-sync sync -m "Nightly sync" >> $HOME/claude-code-sync-cron.log 2>&1
```

Without a terminal nothing can ask you:

- An artifact both machines changed keeps this machine's version and warns.
- A transcript that won't merge keeps the local copy and saves the remote one beside it.
- A git-level conflict stops the pull.

Add `CLAUDE_CODE_SYNC_NO_UPDATE=1` to stay on one version.

## Remote repository access

claude-code-sync runs `git`, so it uses whatever access git already has. **Test with plain git first:** `git -C <sync repo> push origin <branch>`.

### SSH (recommended)

1. Create a key, if you have none:

   ```bash
   ssh-keygen -t ed25519 -C "you@example.com"
   ```

2. Load it into the agent:

   ```bash
   eval "$(ssh-agent -s)"
   ssh-add ~/.ssh/id_ed25519
   ```

3. Add `~/.ssh/id_ed25519.pub` to your host: [GitHub](https://github.com/settings/ssh/new) or [GitLab](https://gitlab.com/-/profile/keys).

4. Test:

   ```bash
   ssh -T git@github.com
   ```

5. Use the `git@…` URL in `init`, or switch an existing repo: `claude-code-sync remote set git@github.com:you/claude-history.git`.

### HTTPS

Use a personal access token as the password (GitHub: a classic token with `repo` scope). Pick a credential helper so git remembers it:

| Platform | Command |
|---|---|
| macOS | `git config --global credential.helper osxkeychain` |
| Linux | `git config --global credential.helper /usr/share/doc/git/contrib/credential/libsecret/git-credential-libsecret` (needs `libsecret`) |
| WSL | `git config --global credential.helper "/mnt/c/Program\ Files/Git/mingw64/bin/git-credential-manager.exe"` |
| Any, in memory | `git config --global credential.helper 'cache --timeout=28800'` |
| Any, on disk | `git config --global credential.helper store` (plain text in `~/.git-credentials`) |

Then push once by hand to store the token: `git -C <sync repo> push origin <branch>`.

## Troubleshooting

| What you see | Cause | Fix |
|---|---|---|
| Pull takes no transcripts | No project mapped, `mapped_projects_only = true` | `config --map-project <path>` |
| `Failed to push: authentication required` / `could not read Username` | Git has no credentials | Set up [SSH or a credential helper](#remote-repository-access) |
| `Permission denied (publickey)` | Key not loaded | `eval "$(ssh-agent -s)" && ssh-add ~/.ssh/id_ed25519` |
| Push rejected by branch protection | The host blocks pushes to that branch | Lift the protection, or sync another branch with `-b` |
| Pull stops on a conflict in cron or CI | No terminal to ask in | Run `claude-code-sync pull` in a terminal once |
| `the sync repository has uncommitted changes` on purge | Unpushed changes in the sync repo | `claude-code-sync push`, then purge |
| `git-lfs is not installed` | LFS on, binary missing | Install `git-lfs` |
| Conflicts on every pull | One session open on two machines at once, clock skew, or different filters per machine | Sync before switching machines; align filters |
| Network error on push | Remote unreachable | `git ls-remote <url>`; check `git -C <sync repo> remote -v` |

More detail: `RUST_LOG=debug`, or `GIT_TRACE=1 GIT_CURL_VERBOSE=1 claude-code-sync push` for git itself.

## Security

- Transcripts can hold anything you pasted into Claude Code. Use a private repository.
- Prefer SSH keys with a passphrase over stored tokens.
- `plans` and `hooks` carry extra risk; see [Artifact sync](#artifact-sync).

## Contributing

Development setup and workflow: [CONTRIBUTING.md](CONTRIBUTING.md). Internals: [docs/architecture.md](docs/architecture.md).
