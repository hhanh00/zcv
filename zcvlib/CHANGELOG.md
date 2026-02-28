# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0](https://github.com/hhanh00/zcv/compare/zcvlib-v0.4.0...zcvlib-v0.5.0) (2026-02-28)


### Features

* add get_election rpc to vote server ([#131](https://github.com/hhanh00/zcv/issues/131)) ([564ab39](https://github.com/hhanh00/zcv/commit/564ab39dd6ab22f29dde4cf1c03e1ed469271d36))
* add scan progress reporter ([#133](https://github.com/hhanh00/zcv/issues/133)) ([44bb541](https://github.com/hhanh00/zcv/commit/44bb5412f3ec71240a8b02661c4cad260b644709))
* **client:** delete election ([#135](https://github.com/hhanh00/zcv/issues/135)) ([0169edf](https://github.com/hhanh00/zcv/commit/0169edf8dfe5c395011e23df778c878801cb7360))


### Bug Fixes

* add a prefix v_ to every table ([#130](https://github.com/hhanh00/zcv/issues/130)) ([a141866](https://github.com/hhanh00/zcv/commit/a141866f21a43308c2c32a3fd5abae731328be8d))
* avoid scanning blocks if they were scanned before ([#134](https://github.com/hhanh00/zcv/issues/134)) ([d5ee50e](https://github.com/hhanh00/zcv/commit/d5ee50e19daefa1a42e476c40bc4829586774153))


### Chores

* remove flutter rust bridge ([#128](https://github.com/hhanh00/zcv/issues/128)) ([bd9fdcd](https://github.com/hhanh00/zcv/commit/bd9fdcd82ecb992d47deed117a8c0c970d47d1a3))

## [0.4.0](https://github.com/hhanh00/zcv/compare/zcvlib-v0.3.0...zcvlib-v0.4.0) (2026-02-19)


### Features

* add command line parser to voter-cometbft ([#94](https://github.com/hhanh00/zcv/issues/94)) ([ec8c015](https://github.com/hhanh00/zcv/commit/ec8c01570e4bb58ce04406812fb87a32e256f16e))
* add memo encryption ([#65](https://github.com/hhanh00/zcv/issues/65)) ([f694cbc](https://github.com/hhanh00/zcv/commit/f694cbcf98c20c616d0bd835bc3a62406ce6ce72))
* add support for dynamically added validators ([#71](https://github.com/hhanh00/zcv/issues/71)) ([9faacfc](https://github.com/hhanh00/zcv/commit/9faacfc0988ad42eb95e31a38b8223931e3d3786))
* **auditor:** collect results ([#110](https://github.com/hhanh00/zcv/issues/110)) ([1bc5a3e](https://github.com/hhanh00/zcv/commit/1bc5a3e1091babcc48b13b02d7450ca03b192639))
* **auditor:** decode ballots ([#109](https://github.com/hhanh00/zcv/issues/109)) ([7469ddf](https://github.com/hhanh00/zcv/commit/7469ddfff225035ccf589fd04f1bd998b6c2b012))
* **auditor:** scan ballots and decrypt data ([#100](https://github.com/hhanh00/zcv/issues/100)) ([eef9e9f](https://github.com/hhanh00/zcv/commit/eef9e9fc6a8623f8b13e685f45db58a112e784a1))
* binary ballot serialization ([#67](https://github.com/hhanh00/zcv/issues/67)) ([4005433](https://github.com/hhanh00/zcv/commit/4005433531bf78a61e2a797e25d6f994fc6b7a12))
* collapse votes to a question to a single address ([#63](https://github.com/hhanh00/zcv/issues/63)) ([d5db8cf](https://github.com/hhanh00/zcv/commit/d5db8cff08f75cb79bbdd74a88f9c5fccecf6765))
* detect spends/delegation during scan ballots ([#107](https://github.com/hhanh00/zcv/issues/107)) ([3086f36](https://github.com/hhanh00/zcv/commit/3086f36f19990fc5010664d4160ed0fa03e38545))
* duplicate nullifier detection ([#111](https://github.com/hhanh00/zcv/issues/111)) ([997a42e](https://github.com/hhanh00/zcv/commit/997a42ed4e85f2abf7542989f5afa889be46b7e4))
* election creator cli ([#123](https://github.com/hhanh00/zcv/issues/123)) ([084b41f](https://github.com/hhanh00/zcv/commit/084b41fa67bad81ea9a50b4dea68457bf2de7b72))
* expose validator API to GRPC ([#73](https://github.com/hhanh00/zcv/issues/73)) ([fd3e76b](https://github.com/hhanh00/zcv/commit/fd3e76bfe09f7a27f636eee07dfff9c21fcd6aeb))
* extended election format ([#83](https://github.com/hhanh00/zcv/issues/83)) ([604c842](https://github.com/hhanh00/zcv/commit/604c842732adaf6237f160e9294c53144d35c08b))
* get_address of account api ([#104](https://github.com/hhanh00/zcv/issues/104)) ([305beb8](https://github.com/hhanh00/zcv/commit/305beb8da01187a4b9c3df197697a412ac0a0e9c))
* grpc vote service ([#69](https://github.com/hhanh00/zcv/issues/69)) ([eaa231f](https://github.com/hhanh00/zcv/commit/eaa231f6c179990fa34b1c886b11cf6ce90c732b))
* improve error message reporting of add_validator ([#76](https://github.com/hhanh00/zcv/issues/76)) ([d8213f2](https://github.com/hhanh00/zcv/commit/d8213f20f27126298b0beecebe02877f0204f234))
* mint & delegation api ([#103](https://github.com/hhanh00/zcv/issues/103)) ([7155839](https://github.com/hhanh00/zcv/commit/7155839d4518b272bbb69526a217cc55f18d8284))
* plurality voting ([#66](https://github.com/hhanh00/zcv/issues/66)) ([e228ce3](https://github.com/hhanh00/zcv/commit/e228ce3450dd0576b199a762f0695eaf4d88d0eb))
* query blocks api ([#98](https://github.com/hhanh00/zcv/issues/98)) ([cfcebe0](https://github.com/hhanh00/zcv/commit/cfcebe01f40c0c2583447f1d10abb6df419b85d0))
* query blocks api ([#99](https://github.com/hhanh00/zcv/issues/99)) ([13b3f2a](https://github.com/hhanh00/zcv/commit/13b3f2a67c7d5be8c9fdf3451718be07e1c99de2))
* scan blocks mutation ([#89](https://github.com/hhanh00/zcv/issues/89)) ([71d12e3](https://github.com/hhanh00/zcv/commit/71d12e32f59a79f3b6884f7c1d18fb49995fd652))
* set the election data via GRPC ([#79](https://github.com/hhanh00/zcv/issues/79)) ([200004c](https://github.com/hhanh00/zcv/commit/200004cc3cfb911fb2f0979de4998f7139ef1488))
* set voting seed api ([#82](https://github.com/hhanh00/zcv/issues/82)) ([3110a23](https://github.com/hhanh00/zcv/commit/3110a238f5ffd9b61e57994ebb170fadeac3bf70))
* set_election rpc (rpc only) ([#77](https://github.com/hhanh00/zcv/issues/77)) ([d291f61](https://github.com/hhanh00/zcv/commit/d291f61de7e2d6db4362e71141018fdd8a551377))
* store current sync height & note position in election table ([#96](https://github.com/hhanh00/zcv/issues/96)) ([8476d28](https://github.com/hhanh00/zcv/commit/8476d28300313eca2a32294620f4d66966800e2b))
* support scanning multiple accounts at once ([#117](https://github.com/hhanh00/zcv/issues/117)) ([16d8f9c](https://github.com/hhanh00/zcv/commit/16d8f9cef3c48df6df3b3820c0366c6efebcb3e8))
* support scanning multiple accounts at once ([#118](https://github.com/hhanh00/zcv/issues/118)) ([919d6af](https://github.com/hhanh00/zcv/commit/919d6af3f2a1056dcfe3d5e987b397a80f1f19fa))
* **ui:** flutter ui ([#85](https://github.com/hhanh00/zcv/issues/85)) ([e53fde4](https://github.com/hhanh00/zcv/commit/e53fde4de07212dee9951382f7be6dfff849ebb8))
* use figment for config file parsing ([#70](https://github.com/hhanh00/zcv/issues/70)) ([099141b](https://github.com/hhanh00/zcv/commit/099141b1237ba01717e748c0b63bb93896a5b2c5))
* voter app graphql cli ([#81](https://github.com/hhanh00/zcv/issues/81)) ([25bb419](https://github.com/hhanh00/zcv/commit/25bb4197de9100afe06fdcfcf834824dccd1635f))
* Voter CI ([#90](https://github.com/hhanh00/zcv/issues/90)) ([3047da7](https://github.com/hhanh00/zcv/commit/3047da712c3a4ab6087857f1a9cffffaa9af3ef9))
* **voter:** integration tests ([#88](https://github.com/hhanh00/zcv/issues/88)) ([62f6e20](https://github.com/hhanh00/zcv/commit/62f6e20eb1ad8bcf74fc0bdfdfe63189bfbb3dc0))
* **voter:** vote method ([#91](https://github.com/hhanh00/zcv/issues/91)) ([ecb0558](https://github.com/hhanh00/zcv/commit/ecb05586fde8e8f0823f0b96facd93451302f8f1))


### Bug Fixes

* ballot output creation ([#101](https://github.com/hhanh00/zcv/issues/101)) ([b88790a](https://github.com/hhanh00/zcv/commit/b88790a8c465cd294609cca7becaae376429a83d))
* ballot vote should only go to associated question ([#102](https://github.com/hhanh00/zcv/issues/102)) ([645cdb8](https://github.com/hhanh00/zcv/commit/645cdb824e748a372726346b4f8f60c2eb8aba65))
* cometbft integration test ([#68](https://github.com/hhanh00/zcv/issues/68)) ([5aae7c9](https://github.com/hhanh00/zcv/commit/5aae7c941ce169b0de35ede0e015a1101104ea01))
* get_balance ([#106](https://github.com/hhanh00/zcv/issues/106)) ([b5e9364](https://github.com/hhanh00/zcv/commit/b5e93646b4c243964d4190b5a0bd055ad2e614c6))
* GRPC GetVoteRange ([#97](https://github.com/hhanh00/zcv/issues/97)) ([c3c4864](https://github.com/hhanh00/zcv/commit/c3c4864c87f87f267e0e8e31f195c61c4d399c18))
* improve double spend detection ([#112](https://github.com/hhanh00/zcv/issues/112)) ([6e5ecc3](https://github.com/hhanh00/zcv/commit/6e5ecc3ee322e26797e97574d4d595bf8337b12a))
* regen Context constructor ([#116](https://github.com/hhanh00/zcv/issues/116)) ([716726e](https://github.com/hhanh00/zcv/commit/716726ec21e772aeb8232461a3bb9c4bb4c4893f))
* remove tally ballots ([#108](https://github.com/hhanh00/zcv/issues/108)) ([d19f32e](https://github.com/hhanh00/zcv/commit/d19f32e714e7ea84de900e689ef44b48361a1dc5))
* scan_ballots should store progression ([#105](https://github.com/hhanh00/zcv/issues/105)) ([3cbbe6e](https://github.com/hhanh00/zcv/commit/3cbbe6e156b30ceebd7851d6dc60ed8a3c83961c))
* test ballot ([#72](https://github.com/hhanh00/zcv/issues/72)) ([5b3f98e](https://github.com/hhanh00/zcv/commit/5b3f98ece651145400a6c7be177b50e98458d3b4))
* use async aware mutex ([#113](https://github.com/hhanh00/zcv/issues/113)) ([c777664](https://github.com/hhanh00/zcv/commit/c777664f5001c4cfbdef6cf76b65d26b0c1f8f8d))


### Chores

* add set_election test ([#78](https://github.com/hhanh00/zcv/issues/78)) ([de7872e](https://github.com/hhanh00/zcv/commit/de7872e64215ed69a65cbcc70cf10a95013bcddd))
* end to end testing in typescript ([#120](https://github.com/hhanh00/zcv/issues/120)) ([3625ae9](https://github.com/hhanh00/zcv/commit/3625ae9a5d49af0e234c19fcf86ba49b092e184c))
* regen dart bindings ([#115](https://github.com/hhanh00/zcv/issues/115)) ([38e4cc0](https://github.com/hhanh00/zcv/commit/38e4cc0ed53d8a77bd3300638efcdf8f92a44fc4))
* scale tests ([#119](https://github.com/hhanh00/zcv/issues/119)) ([7e4207f](https://github.com/hhanh00/zcv/commit/7e4207f4f3f4a8fd7f691f98364e7bc76c568d4a))
* update cometbft version ([#74](https://github.com/hhanh00/zcv/issues/74)) ([7708482](https://github.com/hhanh00/zcv/commit/770848282c5686ea801004901c912f3ac21f5ff7))

## [0.3.0](https://github.com/hhanh00/zcv/compare/zcvlib-v0.2.0...zcvlib-v0.3.0) (2026-01-24)


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
