# Changelog

## [0.5.0](https://github.com/FluffyDiscord/claude-code-sync/compare/v0.4.0...v0.5.0) (2026-10-02)


### Features

* **ci:** add github actions for tests ([6c148bb](https://github.com/FluffyDiscord/claude-code-sync/commit/6c148bb6bb001dd2df9a1ba913c43d70f4018c60))
* **ci:** add renovate github action ([848529a](https://github.com/FluffyDiscord/claude-code-sync/commit/848529a620d1b7b6f6a26ecc1abb4d170127db70))
* **ci:** cleanup all the unused os tests ([f078a58](https://github.com/FluffyDiscord/claude-code-sync/commit/f078a58fdd9adef3c78297478ed4bd4ee9f4d0b8))
* **ci:** resolve issue with tests fighting for config directory ([700dbb6](https://github.com/FluffyDiscord/claude-code-sync/commit/700dbb6a3ebeeee0953466fe853b7a49d3b8d04d))
* **cli:** add docstrings to functions, add interactive conflict resolution ([4f1b7ac](https://github.com/FluffyDiscord/claude-code-sync/commit/4f1b7ac0e917403a6a47c45b518ccb7402ddb4cc))
* **cli:** also do a better job of managing flags and onboarding ([c9eda92](https://github.com/FluffyDiscord/claude-code-sync/commit/c9eda92e620e66c49e72b61262b19327d74ac8f0))
* **cli:** provide more interactive options for the cli ([47498f3](https://github.com/FluffyDiscord/claude-code-sync/commit/47498f336ac20a61dff2ee9714cbbb0280727090))
* **cli:** show prettier output ([600ccf9](https://github.com/FluffyDiscord/claude-code-sync/commit/600ccf9e653db2c80295b70c5287518644eb32f4))
* **cli:** update cli flags for "local" and "remote" ([2abe416](https://github.com/FluffyDiscord/claude-code-sync/commit/2abe4160e0983517f998a863275cc48e4d79afee))
* **config:** completely overhaul how config is managed ([7919b92](https://github.com/FluffyDiscord/claude-code-sync/commit/7919b92110318f87a09b3a12450d96dd3083658e))
* **config:** overhaul what is possible with `config` subcommand ([43d19c7](https://github.com/FluffyDiscord/claude-code-sync/commit/43d19c7191e9e1deba79aba86ec93f9f72c94b67))
* **dev:** clean up unused markdown files ([afb028c](https://github.com/FluffyDiscord/claude-code-sync/commit/afb028ca5e94e209fee8e2d1f6fed913aae06a20))
* **dev:** continue rename and setup github pages for documentation ([4531ff9](https://github.com/FluffyDiscord/claude-code-sync/commit/4531ff967ae050b96c78703b61b901ee34ff0ce6))
* **dev:** create dockerfile ([6d74dd1](https://github.com/FluffyDiscord/claude-code-sync/commit/6d74dd1c94f1bb401f0342b2d565ec9144254ddd))
* **dev:** make sure that `clippy` is happy ([2aab391](https://github.com/FluffyDiscord/claude-code-sync/commit/2aab391c170d1b6d381c9a9a41980c5b1811d2c1))
* **dev:** remove renovate github action since we're using the github app ([ab93e25](https://github.com/FluffyDiscord/claude-code-sync/commit/ab93e25a9d2e74fce94d4c683372c2a3edabf8d3))
* **dev:** rename from `claude-sync` to `claude-code-sync` ([3b7ab04](https://github.com/FluffyDiscord/claude-code-sync/commit/3b7ab04046438a5ec1b0f24c4be3b194c500569e))
* **dev:** update renovate for automerge ([563e614](https://github.com/FluffyDiscord/claude-code-sync/commit/563e614279f63e89c80a377e22f0de6778dc8ddc))
* **docker:** update dockerfile ([bca53d3](https://github.com/FluffyDiscord/claude-code-sync/commit/bca53d3693d008c9fc080dc3157ab96c3327aac1))
* **docs:** update docs and resolve issues with release flow ([529d54b](https://github.com/FluffyDiscord/claude-code-sync/commit/529d54b486540800b6ff82bfbaa3b68d9b57636d))
* **everything:** allow for `undo` and add real integration test data ([4bb51a9](https://github.com/FluffyDiscord/claude-code-sync/commit/4bb51a97aa18f702929d8e4fc9fff0e76634332d))
* **everything:** break apart the large files into smaller ones ([daa4f8e](https://github.com/FluffyDiscord/claude-code-sync/commit/daa4f8e79f31d14995ba7b68f40aea80c516422f))
* **merge:** add "smart merge" capabilities ([86a9c16](https://github.com/FluffyDiscord/claude-code-sync/commit/86a9c16354eb35fefe8c772ea067e7fba7c03191))
* **merge:** improve "smart" merge UUID management ([b7e112e](https://github.com/FluffyDiscord/claude-code-sync/commit/b7e112ef2c199dd3b6b4edd906b0b1ca9ff04084))
* one-line installers, package-manager channels, ARM64 builds and self-update ([#98](https://github.com/FluffyDiscord/claude-code-sync/issues/98)) ([0278e84](https://github.com/FluffyDiscord/claude-code-sync/commit/0278e84103025edfe8c0223186249cba341fba1e))
* **output:** update output to have some better colors/clarification, and throw warnings when values that aren't exported, are set ([b2d84a8](https://github.com/FluffyDiscord/claude-code-sync/commit/b2d84a84738fd13d99b561a1c883412d04b83cc2))
* **projects:** closes [#32](https://github.com/FluffyDiscord/claude-code-sync/issues/32), implements `use_project_name_only` ([3e2e459](https://github.com/FluffyDiscord/claude-code-sync/commit/3e2e459cb1b961781595f59f0f07717c2bb714ee))
* **projects:** closes [#32](https://github.com/FluffyDiscord/claude-code-sync/issues/32), implements `use_project_name_only` ([22dd4e4](https://github.com/FluffyDiscord/claude-code-sync/commit/22dd4e4e61c60609ab2fa0f31e36416be6d55568))
* **readme:** add demo to readme ([566892b](https://github.com/FluffyDiscord/claude-code-sync/commit/566892bd8bdc7c669d841101de7b9013e5fb9f37))
* **readme:** upadte readme for new commands ([79eafd0](https://github.com/FluffyDiscord/claude-code-sync/commit/79eafd00bfd1c79b5ac86c6cd6829b9c4df44d41))
* settle files both machines changed during a pull, one prompt per file ([18dbc90](https://github.com/FluffyDiscord/claude-code-sync/commit/18dbc90abf4f8b8710c53ff5b2412f6ecbad3e3f))
* **snapshots:** better handle large snapshots ([65c9b31](https://github.com/FluffyDiscord/claude-code-sync/commit/65c9b31e8e12d807dd495cab76e7119c8086f48e))
* **snapshots:** better manage snapshots so they're smaller in size ([2520a69](https://github.com/FluffyDiscord/claude-code-sync/commit/2520a6952c6c01bbd7515a6a69aa7f7b2bd55f39))
* sync settings, skills, and other Claude Code artifacts; fix undo push; make attachments real ([#91](https://github.com/FluffyDiscord/claude-code-sync/issues/91)) ([dab13af](https://github.com/FluffyDiscord/claude-code-sync/commit/dab13afe48f56cbe7e9d7d82b5ea96568c68191b))
* **sync:** allow for pull + then push with new sync commmand ([ba87541](https://github.com/FluffyDiscord/claude-code-sync/commit/ba875410506831227ac6655f943d7f156bac145b))
* **tests:** add thorough onboarding config tests ([6403893](https://github.com/FluffyDiscord/claude-code-sync/commit/64038932d1bdd7e45118ff5d452f39099d1fbd90))
* **tests:** also add tests for onboarding ([e7985ab](https://github.com/FluffyDiscord/claude-code-sync/commit/e7985ab76306a9f207d03277b26ac8581f0f50e8))
* **tests:** also create unit/integration tests for export ([4d76cee](https://github.com/FluffyDiscord/claude-code-sync/commit/4d76cee0b827b016f26fa4fb4fba5fa1bb3841da))
* **tests:** provide better unit tests for interactive functionality ([2b0415e](https://github.com/FluffyDiscord/claude-code-sync/commit/2b0415ea96b0a203056935f344aa9206f71bee8f))
* **tui/cli:** introduce onboarding experience, tests, and better logging ([a3af0df](https://github.com/FluffyDiscord/claude-code-sync/commit/a3af0df944f73ca9dc7ea4e17fbd85052f0ce014))


### Bug Fixes

* **ci:** don't error in ci for formatting ([b869a98](https://github.com/FluffyDiscord/claude-code-sync/commit/b869a98a7dad4b87074aaac0077b09771f7641d9))
* **ci:** just have it warn on cargo fmt ([73e4ae2](https://github.com/FluffyDiscord/claude-code-sync/commit/73e4ae2db856f2c43b2c7a371400aced1e36556f))
* **ci:** just skip cargo-audit install if it's already there ([d01f4f4](https://github.com/FluffyDiscord/claude-code-sync/commit/d01f4f4d371403aab04acb55a4c7ca64370a9897))
* **config:** recover the config directory if we can find it ([c6f6006](https://github.com/FluffyDiscord/claude-code-sync/commit/c6f6006b43ed608ad3af69775a079bec1b69f54c))
* convert recursive tree traversal to iterative to prevent stack overflow ([988201a](https://github.com/FluffyDiscord/claude-code-sync/commit/988201a66b6124a29cb8d662858821fdf7b667ce))
* **deps:** update rust crate anyhow to 1.0.102 ([#50](https://github.com/FluffyDiscord/claude-code-sync/issues/50)) ([de71615](https://github.com/FluffyDiscord/claude-code-sync/commit/de716156f6f8483e6448ddd8b5e890b1e3f35f41))
* **deps:** update rust crate chrono to 0.4.43 ([d7b73c5](https://github.com/FluffyDiscord/claude-code-sync/commit/d7b73c5e7d98dd832ad90fb3245909d977262502))
* **deps:** update rust crate chrono to 0.4.43 ([cb61eea](https://github.com/FluffyDiscord/claude-code-sync/commit/cb61eeacc935db887ea389c8f7c7c32fb6da8d9a))
* **deps:** update rust crate chrono to 0.4.44 ([#57](https://github.com/FluffyDiscord/claude-code-sync/issues/57)) ([b1f2674](https://github.com/FluffyDiscord/claude-code-sync/commit/b1f26740c99c2c1ec04905dd7ccea8d3d6a920ef))
* **deps:** update rust crate chrono to 0.4.45 ([#83](https://github.com/FluffyDiscord/claude-code-sync/issues/83)) ([c80e1dd](https://github.com/FluffyDiscord/claude-code-sync/commit/c80e1dd335a6e9f2780c336443cef1a25ed9fa90))
* **deps:** update rust crate clap to 4.5.54 ([d2820e6](https://github.com/FluffyDiscord/claude-code-sync/commit/d2820e671f6e7d4f8dd7cf1babed1fe3fbcba5fb))
* **deps:** update rust crate clap to 4.5.54 ([bdae019](https://github.com/FluffyDiscord/claude-code-sync/commit/bdae01966f80dc2d6922e037051480361ba6b23b))
* **deps:** update rust crate clap to 4.6.0 ([#49](https://github.com/FluffyDiscord/claude-code-sync/issues/49)) ([dfa8acc](https://github.com/FluffyDiscord/claude-code-sync/commit/dfa8acc7dd8579e77ef5214c706b53e3cd493707))
* **deps:** update rust crate clap to 4.6.1 ([#72](https://github.com/FluffyDiscord/claude-code-sync/issues/72)) ([2badf8f](https://github.com/FluffyDiscord/claude-code-sync/commit/2badf8f7574a7a932034642ef0653266c136f0c5))
* **deps:** update rust crate colored to 3.1.1 ([18e2cc5](https://github.com/FluffyDiscord/claude-code-sync/commit/18e2cc5d48be3e99f43dc8eb074a7955d217c749))
* **deps:** update rust crate colored to 3.1.1 ([fd6569c](https://github.com/FluffyDiscord/claude-code-sync/commit/fd6569c7ed76221121387043af57d4276454a68c))
* **deps:** update rust crate env_logger to 0.11.10 ([#64](https://github.com/FluffyDiscord/claude-code-sync/issues/64)) ([af5bf8b](https://github.com/FluffyDiscord/claude-code-sync/commit/af5bf8b180d3c8a4ad2e7461f73933d41dc83813))
* **deps:** update rust crate env_logger to 0.11.9 ([#54](https://github.com/FluffyDiscord/claude-code-sync/issues/54)) ([0cb5876](https://github.com/FluffyDiscord/claude-code-sync/commit/0cb587688a261e9531b01a25677d2aeef47fd69b))
* **deps:** update rust crate inquire to 0.9.2 ([e6bb56a](https://github.com/FluffyDiscord/claude-code-sync/commit/e6bb56afdb9471eab26ac87ef1f3f36fe0be9755))
* **deps:** update rust crate inquire to 0.9.2 ([67d74af](https://github.com/FluffyDiscord/claude-code-sync/commit/67d74af977b982f066e1acc6a28dfe566cd3a63c))
* **deps:** update rust crate inquire to 0.9.4 ([#51](https://github.com/FluffyDiscord/claude-code-sync/issues/51)) ([dcdcb4e](https://github.com/FluffyDiscord/claude-code-sync/commit/dcdcb4e9ff796e7561e7828cbbd9164059f24bd6))
* **deps:** update rust crate log to 0.4.29 ([6057d6e](https://github.com/FluffyDiscord/claude-code-sync/commit/6057d6e2e3a61fda2a41b44aad7fbeb8641f65b9))
* **deps:** update rust crate log to 0.4.29 ([1c7c44a](https://github.com/FluffyDiscord/claude-code-sync/commit/1c7c44abd4c720f2fd08dea15a9c4465489ae6b0))
* **deps:** update rust crate log to 0.4.30 ([#76](https://github.com/FluffyDiscord/claude-code-sync/issues/76)) ([7a24451](https://github.com/FluffyDiscord/claude-code-sync/commit/7a244511f101fa28c4825c586d125c3f540d809a))
* **deps:** update rust crate log to 0.4.31 ([#80](https://github.com/FluffyDiscord/claude-code-sync/issues/80)) ([9c8f5f2](https://github.com/FluffyDiscord/claude-code-sync/commit/9c8f5f255146324e598659304bbf3aba45ca3a3c))
* **deps:** update rust crate log to 0.4.32 ([#82](https://github.com/FluffyDiscord/claude-code-sync/issues/82)) ([93e9fa9](https://github.com/FluffyDiscord/claude-code-sync/commit/93e9fa9042fa79d6b5ac132de6f6f137460516b4))
* **deps:** update rust crate log to 0.4.33 ([#87](https://github.com/FluffyDiscord/claude-code-sync/issues/87)) ([2ad1e74](https://github.com/FluffyDiscord/claude-code-sync/commit/2ad1e745bc529f2e6380bf034336248ca7f1adae))
* **deps:** update rust crate serde_json to 1.0.149 ([7eafa66](https://github.com/FluffyDiscord/claude-code-sync/commit/7eafa66d092593215ade0507a30342dd5c05c030))
* **deps:** update rust crate serde_json to 1.0.149 ([227a094](https://github.com/FluffyDiscord/claude-code-sync/commit/227a0948b5ed7479e0a898e8f3e83169b309b101))
* **deps:** update rust crate serde_json to 1.0.150 ([#75](https://github.com/FluffyDiscord/claude-code-sync/issues/75)) ([8a8c40b](https://github.com/FluffyDiscord/claude-code-sync/commit/8a8c40b2ba3f21c3f5c3a59f14fd0e9690c8902a))
* **deps:** update rust crate toml to 0.9.11 ([68a636a](https://github.com/FluffyDiscord/claude-code-sync/commit/68a636ad64dd294c1b240c77cfb2988f03ff3784))
* **deps:** update rust crate toml to 0.9.11 ([67147f5](https://github.com/FluffyDiscord/claude-code-sync/commit/67147f53b6b115881a06a0db52b939d904df94ff))
* **deps:** update rust crate toml to 0.9.12 ([#53](https://github.com/FluffyDiscord/claude-code-sync/issues/53)) ([3944b1d](https://github.com/FluffyDiscord/claude-code-sync/commit/3944b1daccefd156abcda67ce33b65f47409cb05))
* **deps:** update rust crate toml to v1 ([316845d](https://github.com/FluffyDiscord/claude-code-sync/commit/316845d522522d1197a697e8497e66125ea84154))
* **deps:** update rust crate toml to v1 ([193aac1](https://github.com/FluffyDiscord/claude-code-sync/commit/193aac17329d027f5266327d245ed1ec91887832))
* **deps:** update rust crate uuid to 1.19.0 ([0679312](https://github.com/FluffyDiscord/claude-code-sync/commit/067931221b3292b7c633d01b8f9b3ecb5bba1ca7))
* **deps:** update rust crate uuid to 1.19.0 ([aea032e](https://github.com/FluffyDiscord/claude-code-sync/commit/aea032e9623458b2c1749fa3b779c657730fa524))
* **deps:** update rust crate uuid to 1.22.0 ([#48](https://github.com/FluffyDiscord/claude-code-sync/issues/48)) ([a9fc302](https://github.com/FluffyDiscord/claude-code-sync/commit/a9fc30275d214dc1b5d6fec778271ddeac57bb3d))
* **deps:** update rust crate uuid to 1.23.0 ([#66](https://github.com/FluffyDiscord/claude-code-sync/issues/66)) ([3f9c72d](https://github.com/FluffyDiscord/claude-code-sync/commit/3f9c72dfd2b6b9aaec21d90874949494b5d88278))
* **deps:** update rust crate uuid to 1.23.1 ([#73](https://github.com/FluffyDiscord/claude-code-sync/issues/73)) ([4b755dc](https://github.com/FluffyDiscord/claude-code-sync/commit/4b755dc5caea43c469b2014f0d00255b4b81566d))
* **deps:** update rust crate uuid to 1.23.2 ([#78](https://github.com/FluffyDiscord/claude-code-sync/issues/78)) ([ed5e6df](https://github.com/FluffyDiscord/claude-code-sync/commit/ed5e6df90294e018abc303b258b9d5c2ba76e2e4))
* **deps:** update rust crate uuid to 1.23.3 ([#84](https://github.com/FluffyDiscord/claude-code-sync/issues/84)) ([1022564](https://github.com/FluffyDiscord/claude-code-sync/commit/1022564a6b1e8bf190e042e06ef06022defbdc44))
* **lint:** resolve all clippy warnings across lib, bin, and test targets ([de6b012](https://github.com/FluffyDiscord/claude-code-sync/commit/de6b012dbed7c313d149e6bb68c8ba0d2873f305))
* **readme:** add to readme ([d9ae9c5](https://github.com/FluffyDiscord/claude-code-sync/commit/d9ae9c5bd92670320435b51999fb56df8424f21c))
* **readme:** fix incorrect repo name in readme ([0a529be](https://github.com/FluffyDiscord/claude-code-sync/commit/0a529be01365581a8aa64d337ba76aa611384722))
* **readme:** fix install steps in readme ([518a30c](https://github.com/FluffyDiscord/claude-code-sync/commit/518a30c49e9b744c9c84bea28b009475d91fd907))
* **renovate:** update renovate "grouping" ([a1020f5](https://github.com/FluffyDiscord/claude-code-sync/commit/a1020f5ddb89b5d0c702e4316017e50f187f2357))
* stop sync from reverting artifacts edited since the last sync ([bb4f6eb](https://github.com/FluffyDiscord/claude-code-sync/commit/bb4f6eb89406216442493a0ca791d13242ef47b3))
* subagent transcripts collapse onto parent session id (drops main transcript on pull) ([#89](https://github.com/FluffyDiscord/claude-code-sync/issues/89)) ([340a036](https://github.com/FluffyDiscord/claude-code-sync/commit/340a036c30cf74b3449c064a3915435b4fcd5be2))
* **sync:** extract project name from cwd to handle hyphens correctly ([bdbf925](https://github.com/FluffyDiscord/claude-code-sync/commit/bdbf92513bc4ffd89bd54ce3e2e1f59d72b9ab9e))
* **sync:** extract project name from cwd to handle hyphens correctly ([c7f2405](https://github.com/FluffyDiscord/claude-code-sync/commit/c7f240569b4674d48c0a550373c629d5241d37da))
* **sync:** surface skipped no-cwd sessions in summary ([21b6e8f](https://github.com/FluffyDiscord/claude-code-sync/commit/21b6e8f625c8c9479954f41581f4a0b5480e5ebe))
* **tests:** resolve issue in logger tests competing ([70635d7](https://github.com/FluffyDiscord/claude-code-sync/commit/70635d705e3638360733c204c4cdaf627e697136))
* Windows compilation error in config_dir() ([6903c00](https://github.com/FluffyDiscord/claude-code-sync/commit/6903c006b7c0d457fdbe79c9e30631712956f5d4))
* Windows compilation error in config_dir() ([50854f0](https://github.com/FluffyDiscord/claude-code-sync/commit/50854f0af4ba084d0707c7f054d9e2b803adcf26))


### Documentation

* update documentation for new features ([7339f3a](https://github.com/FluffyDiscord/claude-code-sync/commit/7339f3ab977aa448c5f663ef8aca107167b00caa))


### Miscellaneous Chores

* adopt release-please + mise-driven CI ([#92](https://github.com/FluffyDiscord/claude-code-sync/issues/92)) ([67fb73b](https://github.com/FluffyDiscord/claude-code-sync/commit/67fb73b94e9dabbc25f0707d16e792c31ef1dcbb))
* bump version to 0.3.2 ([666318f](https://github.com/FluffyDiscord/claude-code-sync/commit/666318ff1ed7e1c0c882518f9c0622822536682d))
* clean up warnings and remove unused trait methods ([43409f1](https://github.com/FluffyDiscord/claude-code-sync/commit/43409f1b21b06fbae161c8c7c14196691716f62c))
* **config:** migrate config renovate.json ([82dd0b1](https://github.com/FluffyDiscord/claude-code-sync/commit/82dd0b14addb4043dfacdf660046009cabb8cd43))
* **config:** migrate renovate config ([2904ab7](https://github.com/FluffyDiscord/claude-code-sync/commit/2904ab7c60f8c61b68dec4db2755d66617f03adc))
* **deps:** update actions/cache action to v6 ([#88](https://github.com/FluffyDiscord/claude-code-sync/issues/88)) ([9a1381b](https://github.com/FluffyDiscord/claude-code-sync/commit/9a1381bb1ce1352ac82b43a17a78334f26b04218))
* **deps:** update actions/checkout action to v7 ([#86](https://github.com/FluffyDiscord/claude-code-sync/issues/86)) ([c57f991](https://github.com/FluffyDiscord/claude-code-sync/commit/c57f9910818a38ed4bde1fb280d5eb5adafda1cf))
* **deps:** update actions/upload-artifact action to v7 ([92348b6](https://github.com/FluffyDiscord/claude-code-sync/commit/92348b6a05722e46b2dfbcef522bd159c447a067))
* **deps:** update actions/upload-artifact action to v7 ([ca62355](https://github.com/FluffyDiscord/claude-code-sync/commit/ca623557168102a9e65371750a546bdc928084f7))
* **deps:** update codecov/codecov-action action to v7 ([#81](https://github.com/FluffyDiscord/claude-code-sync/issues/81)) ([dd11dce](https://github.com/FluffyDiscord/claude-code-sync/commit/dd11dce070a785f9f953852ca80123ad3379b393))
* **deps:** update docker/build-push-action action to v7 ([#60](https://github.com/FluffyDiscord/claude-code-sync/issues/60)) ([1adae47](https://github.com/FluffyDiscord/claude-code-sync/commit/1adae47fd4c7b92eeb0daab613b6e4689cdbb456))
* **deps:** update docker/login-action action to v4 ([#59](https://github.com/FluffyDiscord/claude-code-sync/issues/59)) ([7b10c75](https://github.com/FluffyDiscord/claude-code-sync/commit/7b10c755640c29e1e7bd6bc9717017399958b28c))
* **deps:** update docker/metadata-action action to v6 ([#61](https://github.com/FluffyDiscord/claude-code-sync/issues/61)) ([d689a22](https://github.com/FluffyDiscord/claude-code-sync/commit/d689a228529b14443ba9cfd26ceb78e8748b1d6e))
* **deps:** update docker/setup-buildx-action action to v4 ([#62](https://github.com/FluffyDiscord/claude-code-sync/issues/62)) ([c3e3500](https://github.com/FluffyDiscord/claude-code-sync/commit/c3e350011b379eab7c9405e4020f77707bbd3252))
* **deps:** update rust crate rstest to 0.26.1 ([9676ca7](https://github.com/FluffyDiscord/claude-code-sync/commit/9676ca7fd935622e693882314d248dceac31efa3))
* **deps:** update rust crate rstest to 0.26.1 ([698aca0](https://github.com/FluffyDiscord/claude-code-sync/commit/698aca010bea7956481e5c0ee4046533e9fdd6ac))
* **deps:** update rust crate serial_test to 3.3.1 ([6656417](https://github.com/FluffyDiscord/claude-code-sync/commit/66564172f5af9156dcff203caa3c85b72b271e54))
* **deps:** update rust crate serial_test to 3.3.1 ([3fb0096](https://github.com/FluffyDiscord/claude-code-sync/commit/3fb0096bd0adcbe7616366df9af4910d734abbc7))
* **deps:** update rust crate serial_test to 3.4.0 ([#56](https://github.com/FluffyDiscord/claude-code-sync/issues/56)) ([d31e884](https://github.com/FluffyDiscord/claude-code-sync/commit/d31e884a640c8c18ec652bf23b93f79a594c4c92))
* **deps:** update rust crate serial_test to 3.5.0 ([#79](https://github.com/FluffyDiscord/claude-code-sync/issues/79)) ([0ca69ef](https://github.com/FluffyDiscord/claude-code-sync/commit/0ca69ef1d6b360cb017db2ead5ffc8722734b1f9))
* **deps:** update rust crate tempfile to 3.24.0 ([8e75940](https://github.com/FluffyDiscord/claude-code-sync/commit/8e7594037f247521923fa4b8a6e60e1591335963))
* **deps:** update rust crate tempfile to 3.24.0 ([6a46546](https://github.com/FluffyDiscord/claude-code-sync/commit/6a465465a5d514cc19533047b9b1491ef75c7278))
* **deps:** update rust crate tempfile to 3.27.0 ([#52](https://github.com/FluffyDiscord/claude-code-sync/issues/52)) ([d697687](https://github.com/FluffyDiscord/claude-code-sync/commit/d6976879796ae000e697680fb232594962753c77))
* **deps:** update rust dependencies ([d67d1c7](https://github.com/FluffyDiscord/claude-code-sync/commit/d67d1c79b35f1cfd60d5684dc048f7a4f1d379da))
* **deps:** update rust dependencies ([5124ae6](https://github.com/FluffyDiscord/claude-code-sync/commit/5124ae689b1caabd1210a397f79d58d6bc490c64))
* **deps:** update rust dependencies ([9566070](https://github.com/FluffyDiscord/claude-code-sync/commit/956607049a34227d24794777988623e9d9aa51c0))
* **deps:** update rust dependencies (major) ([101c960](https://github.com/FluffyDiscord/claude-code-sync/commit/101c9603a639bba427bb0731dcc12fb2cec36f08))
* **deps:** update rust docker tag to v1.95 ([#74](https://github.com/FluffyDiscord/claude-code-sync/issues/74)) ([d9a7c0c](https://github.com/FluffyDiscord/claude-code-sync/commit/d9a7c0c43dd200d18de383e5e25d417e4b1c8e94))
* **deps:** update rust docker tag to v1.96 ([#77](https://github.com/FluffyDiscord/claude-code-sync/issues/77)) ([98a0f6a](https://github.com/FluffyDiscord/claude-code-sync/commit/98a0f6adfe6eb5aeafd4f0e556354f86f68e975c))
* **deps:** update softprops/action-gh-release action to v3 ([#69](https://github.com/FluffyDiscord/claude-code-sync/issues/69)) ([58e29ca](https://github.com/FluffyDiscord/claude-code-sync/commit/58e29ca169d4aa399cf482a7f7193a76b2b223a0))
* **main:** release 0.3.3 ([#95](https://github.com/FluffyDiscord/claude-code-sync/issues/95)) ([ecd1d4d](https://github.com/FluffyDiscord/claude-code-sync/commit/ecd1d4d91c51f53f0f3640066d9b893c84e45667))
* **main:** release 0.4.0 ([#99](https://github.com/FluffyDiscord/claude-code-sync/issues/99)) ([1481808](https://github.com/FluffyDiscord/claude-code-sync/commit/1481808eb3ea2be999455bd4b4008b7776fe11c6))
* make chore/refactor/docs/style commits release-worthy ([#94](https://github.com/FluffyDiscord/claude-code-sync/issues/94)) ([6fb1467](https://github.com/FluffyDiscord/claude-code-sync/commit/6fb14673abc9e73c292542671c963f7f6799aae6))
* update workflow triggers after master -&gt; main rename ([bc04297](https://github.com/FluffyDiscord/claude-code-sync/commit/bc0429703abd0491ac7dffd0add279aeac205efa))


### Code Refactoring

* replace git2 library with CLI-based SCM abstraction ([503da04](https://github.com/FluffyDiscord/claude-code-sync/commit/503da04b0ec59616fafd5ba6bf4ffc035a1bed07))
* replace git2 library with CLI-based SCM abstraction ([76b0d2d](https://github.com/FluffyDiscord/claude-code-sync/commit/76b0d2da4da71d402da1d11af73ca853212fd648))
* reshape three oversized files into focused modules ([#93](https://github.com/FluffyDiscord/claude-code-sync/issues/93)) ([286b3b4](https://github.com/FluffyDiscord/claude-code-sync/commit/286b3b4fb5c833cd2654efafed98f27509e9dc11))

## [0.4.0](https://github.com/perfectra1n/claude-code-sync/compare/v0.3.3...v0.4.0) (2026-09-24)


### Features

* one-line installers, package-manager channels, ARM64 builds and self-update ([#98](https://github.com/perfectra1n/claude-code-sync/issues/98)) ([0278e84](https://github.com/perfectra1n/claude-code-sync/commit/0278e84103025edfe8c0223186249cba341fba1e))

## [0.3.3](https://github.com/perfectra1n/claude-code-sync/compare/v0.3.2...v0.3.3) (2026-07-13)


### Miscellaneous Chores

* adopt release-please + mise-driven CI ([#92](https://github.com/perfectra1n/claude-code-sync/issues/92)) ([67fb73b](https://github.com/perfectra1n/claude-code-sync/commit/67fb73b94e9dabbc25f0707d16e792c31ef1dcbb))
* make chore/refactor/docs/style commits release-worthy ([#94](https://github.com/perfectra1n/claude-code-sync/issues/94)) ([6fb1467](https://github.com/perfectra1n/claude-code-sync/commit/6fb14673abc9e73c292542671c963f7f6799aae6))


### Code Refactoring

* reshape three oversized files into focused modules ([#93](https://github.com/perfectra1n/claude-code-sync/issues/93)) ([286b3b4](https://github.com/perfectra1n/claude-code-sync/commit/286b3b4fb5c833cd2654efafed98f27509e9dc11))
