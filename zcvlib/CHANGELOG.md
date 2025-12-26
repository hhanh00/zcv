# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
