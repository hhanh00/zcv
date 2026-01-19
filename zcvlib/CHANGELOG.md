# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0](https://github.com/hhanh00/zcv/compare/zcvlib-v0.2.0...zcvlib-v0.3.0) (2026-01-19)


### Features

* apphash calculation ([#44](https://github.com/hhanh00/zcv/issues/44)) ([a99daaa](https://github.com/hhanh00/zcv/commit/a99daaa4944978f8dbbf4b5479295f1d945b99de))
* fetch ballots fn ([#48](https://github.com/hhanh00/zcv/issues/48)) ([1125768](https://github.com/hhanh00/zcv/commit/1125768660d3fdb488a645781d5c165200725c8b))
* fetch ballots fn ([#49](https://github.com/hhanh00/zcv/issues/49)) ([217cf60](https://github.com/hhanh00/zcv/commit/217cf6002e2e531b5c595c503e1e9f886c663536))
* store ballot in finalize_block ([#46](https://github.com/hhanh00/zcv/issues/46)) ([daf76af](https://github.com/hhanh00/zcv/commit/daf76afc6fe3d7287aa65e7d52447776c0676cc1))
* submit ballot through cometbft ([#42](https://github.com/hhanh00/zcv/issues/42)) ([0d7d7c4](https://github.com/hhanh00/zcv/commit/0d7d7c4ce083abbff48a00c701a59343a990e297))
* submit_ballot rpc ([#40](https://github.com/hhanh00/zcv/issues/40)) ([379e047](https://github.com/hhanh00/zcv/commit/379e0473d41a61745c1f0d3ea9e2c8c16db2d0f6))
* tally election ballots test ([#52](https://github.com/hhanh00/zcv/issues/52)) ([bbd6fb9](https://github.com/hhanh00/zcv/commit/bbd6fb92527798bbbfb80bafafab195d1acf9df6))
* tally votes ([#50](https://github.com/hhanh00/zcv/issues/50)) ([869c946](https://github.com/hhanh00/zcv/commit/869c946a9e43007611b2b13a35b4c654d356167d))
* test for ballots with large amounts ([#53](https://github.com/hhanh00/zcv/issues/53)) ([b59866c](https://github.com/hhanh00/zcv/commit/b59866c6fefc4a04cfd03732909945486c5a982a))
* test for ballots with large amounts ([#54](https://github.com/hhanh00/zcv/issues/54)) ([0ff936b](https://github.com/hhanh00/zcv/commit/0ff936b51e40924b25b05b80a8018e2a78333b52))


### Bug Fixes

* fetch_ballot with callback instead of returning stream ([#51](https://github.com/hhanh00/zcv/issues/51)) ([3b5594a](https://github.com/hhanh00/zcv/commit/3b5594a9ef4ec3a11963d2cf4bd85759306e281a))
* typo ([#43](https://github.com/hhanh00/zcv/issues/43)) ([3ae83e3](https://github.com/hhanh00/zcv/commit/3ae83e3dc9fd1bda5379bc39a9817efc0b445185))
* use domain hash for db lookups instead of id ([#45](https://github.com/hhanh00/zcv/issues/45)) ([6580454](https://github.com/hhanh00/zcv/commit/65804548b350aed916b3c97619999f033e727bb3))

## [0.2.0](https://github.com/hhanh00/zcv/compare/zcvlib-v0.1.0...zcvlib-v0.2.0) (2025-12-26)


### Features

* ABCI application skeleton ([#29](https://github.com/hhanh00/zcv/issues/29)) ([244aef3](https://github.com/hhanh00/zcv/commit/244aef34b879e2e32f8e82357bcdce825b0aab58))
* add http client to cometbft engine ([#31](https://github.com/hhanh00/zcv/issues/31)) ([fcad6af](https://github.com/hhanh00/zcv/commit/fcad6af975976467e623a45446bd54359f87f48d))
* add more db tables ([#6](https://github.com/hhanh00/zcv/issues/6)) ([3afc00f](https://github.com/hhanh00/zcv/commit/3afc00f3763340a283bb0ae8102b20a28929d1a5))
* add rocket/rpc server ([#34](https://github.com/hhanh00/zcv/issues/34)) ([10610ea](https://github.com/hhanh00/zcv/commit/10610eace69457587aa118beb31d5b67e8f71f1e))
* cometbft app bin ([#33](https://github.com/hhanh00/zcv/issues/33)) ([6dd0bdc](https://github.com/hhanh00/zcv/commit/6dd0bdccf98445e98636e2200adfff5a00829fa9))
* compute initial voting power (balance) ([#21](https://github.com/hhanh00/zcv/issues/21)) ([d791478](https://github.com/hhanh00/zcv/commit/d7914784e74f1b0aa0dd76acd28a5cf8d67256ed))
* create election ([#2](https://github.com/hhanh00/zcv/issues/2)) ([ddb3fac](https://github.com/hhanh00/zcv/commit/ddb3facb92dc7ef7c056bc8052c7b2f3ec1a7341))
* db creation ([#5](https://github.com/hhanh00/zcv/issues/5)) ([4b70f91](https://github.com/hhanh00/zcv/commit/4b70f917c70ed334a21f0bc803dd0b9b5581272a))
* detect spends and store them in database ([#20](https://github.com/hhanh00/zcv/issues/20)) ([2ffc0ac](https://github.com/hhanh00/zcv/commit/2ffc0ac1de2b79865a0a3192ee2f69648b7360c5))
* encrypt ballot data ([#24](https://github.com/hhanh00/zcv/issues/24)) ([f9a932e](https://github.com/hhanh00/zcv/commit/f9a932e25a2101b467d7414e8e35b4ee3de28e26))
* get_blocks ([#8](https://github.com/hhanh00/zcv/issues/8)) ([e5c69e1](https://github.com/hhanh00/zcv/commit/e5c69e10a5f4ba59a4856a585a1314fa4271fda5))
* lwd connector ([#7](https://github.com/hhanh00/zcv/issues/7)) ([1740a4a](https://github.com/hhanh00/zcv/commit/1740a4a4ef4a488c32b783075453b6e5a0927ac5))
* parse election and store in db ([#10](https://github.com/hhanh00/zcv/issues/10)) ([f4bbeb8](https://github.com/hhanh00/zcv/commit/f4bbeb82f2fa864037648ac5825b7a5a2a057c6a))
* scan for notes ([#9](https://github.com/hhanh00/zcv/issues/9)) ([4951e3b](https://github.com/hhanh00/zcv/commit/4951e3bd17bd7bfb16a1a540108ffc1b27ea0efe))
* store ballot in db ([#27](https://github.com/hhanh00/zcv/issues/27)) ([be327b7](https://github.com/hhanh00/zcv/commit/be327b7cc36be97c0027b7dd8d56bdb0bd608259))
* store election seed ([#26](https://github.com/hhanh00/zcv/issues/26)) ([28dec9e](https://github.com/hhanh00/zcv/commit/28dec9e816cbffae738e5e86b9ff97fead754959))
* store incoming notes into db ([#13](https://github.com/hhanh00/zcv/issues/13)) ([05744d2](https://github.com/hhanh00/zcv/commit/05744d29263724a39d67a3b92c7dc0b37956b908))
* try decrypt ballot data ([#23](https://github.com/hhanh00/zcv/issues/23)) ([0f20986](https://github.com/hhanh00/zcv/commit/0f209860ba99c6766da8ad7ea2b99a929fb0cb64))
* vote server rpc ([#30](https://github.com/hhanh00/zcv/issues/30)) ([3a47bc9](https://github.com/hhanh00/zcv/commit/3a47bc924043b4c5f697b72d0ffed19580cd9f5d))


### Bug Fixes

* add data/json column to elections table ([#12](https://github.com/hhanh00/zcv/issues/12)) ([6dd7e9b](https://github.com/hhanh00/zcv/commit/6dd7e9bbcb2bee25e1f06095a6938c579312435d))
* add domain column to questions table ([#11](https://github.com/hhanh00/zcv/issues/11)) ([05704bb](https://github.com/hhanh00/zcv/commit/05704bb47efbbac15cc5af59cd96894a694771a0))
* add domain nullifier column to received notes table ([#19](https://github.com/hhanh00/zcv/issues/19)) ([ea81f5a](https://github.com/hhanh00/zcv/commit/ea81f5a5146888575727a78d090f8e0b1fb43095))
* domain hash ([#25](https://github.com/hhanh00/zcv/issues/25)) ([6c2eda0](https://github.com/hhanh00/zcv/commit/6c2eda0f1ff64b816bea1b163417266739a23006))
* move domain hash out of public data since we cannot trust ([#3](https://github.com/hhanh00/zcv/issues/3)) ([8a28895](https://github.com/hhanh00/zcv/commit/8a288957316fb812cd43017454f462b7eb663c99))

## 0.1.0 (2025-12-26)


### Features

* ABCI application skeleton ([#29](https://github.com/hhanh00/zcv/issues/29)) ([244aef3](https://github.com/hhanh00/zcv/commit/244aef34b879e2e32f8e82357bcdce825b0aab58))
* add http client to cometbft engine ([#31](https://github.com/hhanh00/zcv/issues/31)) ([fcad6af](https://github.com/hhanh00/zcv/commit/fcad6af975976467e623a45446bd54359f87f48d))
* add more db tables ([#6](https://github.com/hhanh00/zcv/issues/6)) ([3afc00f](https://github.com/hhanh00/zcv/commit/3afc00f3763340a283bb0ae8102b20a28929d1a5))
* add rocket/rpc server ([#34](https://github.com/hhanh00/zcv/issues/34)) ([10610ea](https://github.com/hhanh00/zcv/commit/10610eace69457587aa118beb31d5b67e8f71f1e))
* cometbft app bin ([#33](https://github.com/hhanh00/zcv/issues/33)) ([6dd0bdc](https://github.com/hhanh00/zcv/commit/6dd0bdccf98445e98636e2200adfff5a00829fa9))
* compute initial voting power (balance) ([#21](https://github.com/hhanh00/zcv/issues/21)) ([d791478](https://github.com/hhanh00/zcv/commit/d7914784e74f1b0aa0dd76acd28a5cf8d67256ed))
* create election ([#2](https://github.com/hhanh00/zcv/issues/2)) ([ddb3fac](https://github.com/hhanh00/zcv/commit/ddb3facb92dc7ef7c056bc8052c7b2f3ec1a7341))
* db creation ([#5](https://github.com/hhanh00/zcv/issues/5)) ([4b70f91](https://github.com/hhanh00/zcv/commit/4b70f917c70ed334a21f0bc803dd0b9b5581272a))
* detect spends and store them in database ([#20](https://github.com/hhanh00/zcv/issues/20)) ([2ffc0ac](https://github.com/hhanh00/zcv/commit/2ffc0ac1de2b79865a0a3192ee2f69648b7360c5))
* encrypt ballot data ([#24](https://github.com/hhanh00/zcv/issues/24)) ([f9a932e](https://github.com/hhanh00/zcv/commit/f9a932e25a2101b467d7414e8e35b4ee3de28e26))
* get_blocks ([#8](https://github.com/hhanh00/zcv/issues/8)) ([e5c69e1](https://github.com/hhanh00/zcv/commit/e5c69e10a5f4ba59a4856a585a1314fa4271fda5))
* lwd connector ([#7](https://github.com/hhanh00/zcv/issues/7)) ([1740a4a](https://github.com/hhanh00/zcv/commit/1740a4a4ef4a488c32b783075453b6e5a0927ac5))
* parse election and store in db ([#10](https://github.com/hhanh00/zcv/issues/10)) ([f4bbeb8](https://github.com/hhanh00/zcv/commit/f4bbeb82f2fa864037648ac5825b7a5a2a057c6a))
* scan for notes ([#9](https://github.com/hhanh00/zcv/issues/9)) ([4951e3b](https://github.com/hhanh00/zcv/commit/4951e3bd17bd7bfb16a1a540108ffc1b27ea0efe))
* store ballot in db ([#27](https://github.com/hhanh00/zcv/issues/27)) ([be327b7](https://github.com/hhanh00/zcv/commit/be327b7cc36be97c0027b7dd8d56bdb0bd608259))
* store election seed ([#26](https://github.com/hhanh00/zcv/issues/26)) ([28dec9e](https://github.com/hhanh00/zcv/commit/28dec9e816cbffae738e5e86b9ff97fead754959))
* store incoming notes into db ([#13](https://github.com/hhanh00/zcv/issues/13)) ([05744d2](https://github.com/hhanh00/zcv/commit/05744d29263724a39d67a3b92c7dc0b37956b908))
* try decrypt ballot data ([#23](https://github.com/hhanh00/zcv/issues/23)) ([0f20986](https://github.com/hhanh00/zcv/commit/0f209860ba99c6766da8ad7ea2b99a929fb0cb64))
* vote server rpc ([#30](https://github.com/hhanh00/zcv/issues/30)) ([3a47bc9](https://github.com/hhanh00/zcv/commit/3a47bc924043b4c5f697b72d0ffed19580cd9f5d))


### Bug Fixes

* add data/json column to elections table ([#12](https://github.com/hhanh00/zcv/issues/12)) ([6dd7e9b](https://github.com/hhanh00/zcv/commit/6dd7e9bbcb2bee25e1f06095a6938c579312435d))
* add domain column to questions table ([#11](https://github.com/hhanh00/zcv/issues/11)) ([05704bb](https://github.com/hhanh00/zcv/commit/05704bb47efbbac15cc5af59cd96894a694771a0))
* add domain nullifier column to received notes table ([#19](https://github.com/hhanh00/zcv/issues/19)) ([ea81f5a](https://github.com/hhanh00/zcv/commit/ea81f5a5146888575727a78d090f8e0b1fb43095))
* domain hash ([#25](https://github.com/hhanh00/zcv/issues/25)) ([6c2eda0](https://github.com/hhanh00/zcv/commit/6c2eda0f1ff64b816bea1b163417266739a23006))
* move domain hash out of public data since we cannot trust ([#3](https://github.com/hhanh00/zcv/issues/3)) ([8a28895](https://github.com/hhanh00/zcv/commit/8a288957316fb812cd43017454f462b7eb663c99))

## [Unreleased]

## [0.1.0](https://github.com/hhanh00/zcv/releases/tag/v0.1.0) - 2025-12-26

### Added

- add rocket/rpc server ([#34](https://github.com/hhanh00/zcv/pull/34))
- cometbft app bin ([#33](https://github.com/hhanh00/zcv/pull/33))
- add http client to cometbft engine ([#31](https://github.com/hhanh00/zcv/pull/31))
- vote server rpc ([#30](https://github.com/hhanh00/zcv/pull/30))
- ABCI application skeleton ([#29](https://github.com/hhanh00/zcv/pull/29))
- store ballot in db ([#27](https://github.com/hhanh00/zcv/pull/27))
- store election seed ([#26](https://github.com/hhanh00/zcv/pull/26))
- encrypt ballot data ([#24](https://github.com/hhanh00/zcv/pull/24))
- try decrypt ballot data ([#23](https://github.com/hhanh00/zcv/pull/23))
- compute initial voting power (balance) ([#21](https://github.com/hhanh00/zcv/pull/21))
- detect spends and store them in database ([#20](https://github.com/hhanh00/zcv/pull/20))
- store incoming notes into db ([#13](https://github.com/hhanh00/zcv/pull/13))
- parse election and store in db ([#10](https://github.com/hhanh00/zcv/pull/10))
- scan for notes ([#9](https://github.com/hhanh00/zcv/pull/9))
- get_blocks ([#8](https://github.com/hhanh00/zcv/pull/8))
- lwd connector ([#7](https://github.com/hhanh00/zcv/pull/7))
- add more db tables ([#6](https://github.com/hhanh00/zcv/pull/6))
- db creation ([#5](https://github.com/hhanh00/zcv/pull/5))
- create election ([#2](https://github.com/hhanh00/zcv/pull/2))

### Fixed

- domain hash ([#25](https://github.com/hhanh00/zcv/pull/25))
- add domain nullifier column to received notes table ([#19](https://github.com/hhanh00/zcv/pull/19))
- add data/json column to elections table ([#12](https://github.com/hhanh00/zcv/pull/12))
- add domain column to questions table ([#11](https://github.com/hhanh00/zcv/pull/11))
- move domain hash out of public data since we cannot trust ([#3](https://github.com/hhanh00/zcv/pull/3))

### Other

- add logger/tracing ([#32](https://github.com/hhanh00/zcv/pull/32))
- add tendermint dependencies ([#28](https://github.com/hhanh00/zcv/pull/28))
- refactor tests ([#22](https://github.com/hhanh00/zcv/pull/22))
- add release please CI ([#16](https://github.com/hhanh00/zcv/pull/16))
- add context struct ([#4](https://github.com/hhanh00/zcv/pull/4))
- initial version + orchard/vote dependency
