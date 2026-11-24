# TODO

## Bugs

- [ ] `pull --interactive` asks "Pull?" only after `git pull` has merged the remote and the sync attributes are committed. Answering no leaves the sync repo merged; only `~/.claude` stays untouched (`src/sync/pull.rs`).
