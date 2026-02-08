import * as bip39 from 'bip39';

const seed = bip39.generateMnemonic(256)
console.log(seed)
