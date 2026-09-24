# Changelog

## [0.4.0](https://github.com/FluffyDiscord/claude-code-sync/compare/v0.3.3...v0.4.0) (2026-09-24)


### Features

* list each artifact file a push or pull changed ([0468177](https://github.com/FluffyDiscord/claude-code-sync/commit/04681778e3913dda0ab5a02b3c0af47477caf345))
* sync artifacts across machines that differ, and purge old transcripts ([524e147](https://github.com/FluffyDiscord/claude-code-sync/commit/524e147baca3cf8ac30247159fe77a9a340bf921))
* sync hook scripts, and let the executable bit travel ([0322245](https://github.com/FluffyDiscord/claude-code-sync/commit/03222454c3461367e427412fb640cc8df79beb67))


### Bug Fixes

* address review findings on the portability changes ([339ed5f](https://github.com/FluffyDiscord/claude-code-sync/commit/339ed5f3fb8368842f0cf24520dec1361887f810))
* build the release binaries when release-please is skipped ([aa13358](https://github.com/FluffyDiscord/claude-code-sync/commit/aa13358086073ffe7d93b557f00336acdac7b0a7))
* copy exactly the messages a session was summarized from ([fbec291](https://github.com/FluffyDiscord/claude-code-sync/commit/fbec291fc66c779c954e8ad9d021e762aa25966f))
* fold unmapped-project skips into one warning ([2862121](https://github.com/FluffyDiscord/claude-code-sync/commit/286212112fd2b8ecd3a0bd3b6cbca8132f6e0a20))
* keep a session whole when purging, and never delete an uncommitted copy ([6ea7806](https://github.com/FluffyDiscord/claude-code-sync/commit/6ea7806a61ca7011b9229859f208c8bfe1daf921))
* merge append-only files instead of stopping the sync ([fcf1a12](https://github.com/FluffyDiscord/claude-code-sync/commit/fcf1a12078921094e3e36171892e56b60d3278c2))
* pair transcripts by where they live, not by session id ([9218acb](https://github.com/FluffyDiscord/claude-code-sync/commit/9218acbdb41f7e21b266b2a6f69867b37e3d8e25))
* stop a pull the remote cannot be merged into ([61f6a8d](https://github.com/FluffyDiscord/claude-code-sync/commit/61f6a8d0d4996ac4f80139a560690dc110ee8a44))
* store sync repository text files with LF on every machine ([8ab0b1c](https://github.com/FluffyDiscord/claude-code-sync/commit/8ab0b1cf352561224849e34abc8ecd31b290fedf))
* warn when a conversation is skipped for being over the size limit ([4a02b57](https://github.com/FluffyDiscord/claude-code-sync/commit/4a02b578b02751b7af509c7812e2af246006eb6d))


### Performance Improvements

* summarize transcripts instead of holding them, on every core ([14d9912](https://github.com/FluffyDiscord/claude-code-sync/commit/14d99126f4da4b343f99ed30f7a69ac825e988af))


### Documentation

* document the new categories, commands and config keys ([aab2fcd](https://github.com/FluffyDiscord/claude-code-sync/commit/aab2fcd314efcdd0bc6c3d9c674b966078f986b3))


### Miscellaneous Chores

* release 0.4.2 ([cf28549](https://github.com/FluffyDiscord/claude-code-sync/commit/cf28549d9a9146d32097ee2c74b52dceb9fe437f))
* release 0.4.3 ([59486db](https://github.com/FluffyDiscord/claude-code-sync/commit/59486db77b8a0f9d1643fcd5443cf89f0416438c))
* release 0.4.4 ([bce9c40](https://github.com/FluffyDiscord/claude-code-sync/commit/bce9c40001e0168d00b1962c2982b18397c82bfd))
* release 0.4.5 ([b5b41be](https://github.com/FluffyDiscord/claude-code-sync/commit/b5b41be2ab4c0be30673a91c2d383e328d8cb7fc))
* release 0.4.6 ([2d5f338](https://github.com/FluffyDiscord/claude-code-sync/commit/2d5f3386082120d719c2c09a743268afd25f2910))
* release 0.4.7 ([8d4435a](https://github.com/FluffyDiscord/claude-code-sync/commit/8d4435a0d10e43850d9725803159161b197af10b))
* restore the upstream version so release-please picks the next one ([34ad349](https://github.com/FluffyDiscord/claude-code-sync/commit/34ad349c11431d00337b5958ae404777e16dcd40))

## [0.3.3](https://github.com/perfectra1n/claude-code-sync/compare/v0.3.2...v0.3.3) (2026-07-13)


### Miscellaneous Chores

* adopt release-please + mise-driven CI ([#92](https://github.com/perfectra1n/claude-code-sync/issues/92)) ([67fb73b](https://github.com/perfectra1n/claude-code-sync/commit/67fb73b94e9dabbc25f0707d16e792c31ef1dcbb))
* make chore/refactor/docs/style commits release-worthy ([#94](https://github.com/perfectra1n/claude-code-sync/issues/94)) ([6fb1467](https://github.com/perfectra1n/claude-code-sync/commit/6fb14673abc9e73c292542671c963f7f6799aae6))


### Code Refactoring

* reshape three oversized files into focused modules ([#93](https://github.com/perfectra1n/claude-code-sync/issues/93)) ([286b3b4](https://github.com/perfectra1n/claude-code-sync/commit/286b3b4fb5c833cd2654efafed98f27509e9dc11))
