use anyhow::Result;
use pasta_curves::Fp;
use pir_client::{ImtProofData, PirClient};

/// Connect to a remote PIR server and retrieve a Merkle proof for the given nullifier.
///
/// `nullifier` is the 32-byte little-endian representation of the nullifier field element.
/// `server_url` is the base URL of the PIR server (e.g. `"http://localhost:8080"`).
pub async fn query_pir_proof(nullifier: Fp, server_url: &str) -> Result<ImtProofData> {
    let client = PirClient::connect(server_url).await?;
    client.fetch_proof(nullifier).await
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use bincode::config::legacy;
    use ff::PrimeField;
    use orchard::{
        vote::{MerklePathGeneric, NfExclusion, NfExclusionInfo},
    };
    use pasta_curves::Fp;
    use pir_client::{ImtProofData, PirClient};
    use zcash_trees::warp::{
        Witness,
        hasher::{OrchardHasher, empty_roots},
        legacy::CommitmentTreeFrontier,
    };

    use crate::tiu;

    // const SNAPSHOT_HEIGHT: u32 = 3240000;

    fn parse_fp(hex_str: &str) -> Fp {
        let bytes = hex::decode(hex_str).expect("valid hex");
        let mut repr = [0u8; 32];
        repr.copy_from_slice(&bytes);
        Option::from(Fp::from_repr(repr)).expect("valid field element")
    }

    #[test]
    fn vote_pir_real_server_vector_verifies() {
        let proof = ImtProofData {
            root: parse_fp("7667542adf6d69bdac77cec938ae8598a8325bfb6996a64ef37c4ee06456cd1b"),
            low: parse_fp("0100000000000000000000000000000000000000000000000000000000000000"),
            width: parse_fp("d44b03ae60dc31df14d322096fed2f66d5605c96be141e18c8a20ae118000000"),
            leaf_pos: 0,
            path: [
                parse_fp("70271a3ee03398d52b5af6068afca6c7b74d65c0fda6b099a01d09f064410c08"),
                parse_fp("c30c3496fa119e4fb946d8dcd0cdae320078199d857f1b60230502c8d7ea653f"),
                parse_fp("fc2e548d6a51194c96fce1437ba8fc1185ee38c59325b0a0906a60eeefd8ca1c"),
                parse_fp("f46b06ec67d8525f8181621b5cd3c49fa28d83d5cb614ff3a9a0e46e189cb606"),
                parse_fp("50d08462091c85711b301537b871a69091d64087e1c8eb9a41110e3839ca3e1b"),
                parse_fp("c499b1ce86b7f63eb93163ef5d765592b9a22a50caedec91aa9508eb4c0d712b"),
                parse_fp("c111c06e9f14f5ce35d8e4fc38e6d39ce458a00f4bfed67d895e590f07468a08"),
                parse_fp("4e9feee48ca218065e6e807b926a3aee133da1730410a822e361e4a8cc44552a"),
                parse_fp("4d6eb3b4e68fd57a046cdfdeb42246a90b02230fabe0afdb01fffb1c6e969528"),
                parse_fp("a980f13c62a56f47033c32d5549178e3018717c9b417f6cc4c8a747581721415"),
                parse_fp("c942a8875969546058cbd3dd12ad31795a4e153873d829f964ab18feba317310"),
                parse_fp("3f4c7f2dad01000e97364987fe5aad85f59a93aff3ca589b7e787950d3567405"),
                parse_fp("958e13893b5f80a6f1410537c8c6002a117f436e846c3ea014e89df669fa040c"),
                parse_fp("986ae2371d6a18e29f258a259d05214edd001628aaa980250534c244ef7e1503"),
                parse_fp("0274e0b34d25c029ec17b90083a6757554dacfb0269cb687f027b24541037b38"),
                parse_fp("6584319e8415c729e686d0ed4b7e886a295032dbb297913bddf5995c8b4fce21"),
                parse_fp("f3e4044625d34c9e0eab0b33f60b62b54538da820d5dadda60c5dbcb951ad729"),
                parse_fp("7af4f7557e0efc7dc8487d121f0995b4c2c4ed1966fb2fc0c1014f67aed2dd30"),
                parse_fp("d67686f48611015cf8c27f7a1a3a1edced196109186f86881e28d5a801d7d312"),
                parse_fp("f6ff3f6f511cacd0f4fba264403b0050fe6936f72a0ffaad676c3d3c54bcac00"),
                parse_fp("ebc630a19df0a1da7c1d48e96d3324da8455dd9f7675ed65e16657dd151da31d"),
                parse_fp("95f9b702f44d2e061bea1c7928fcb14e4d3eae83ef8506e9ee6c86c6d9515317"),
                parse_fp("35b6a742e7c518e37badf31f02531d064fd2bd5959bd46562c3947c3f81cc005"),
                parse_fp("a5068e4104aa2b7776df9ab98178ca96f3257424a62301576d6105b110e6230e"),
                parse_fp("385d2c36139f325c2af2daa7379350671f565d8ebf894b94eb1755cb649feb1c"),
                parse_fp("f0ff396552f703bfb46e81e5ae21a0a7ead2785a93ee5e35fe1351d4b177303a"),
                parse_fp("e0bdfc8b65bf2a18acfaff5c7199b4b2842c3306c4432a911c88d043df66d224"),
                parse_fp("ce9c8fdec0fdf20c7d9a3f98e2e7e4f67447ec28b8700a19cbbd5e1408e3eb37"),
                parse_fp("761f1f50ffcec37286e3ba4dd03f5d1d3d520a75aee6b142956b1a760283f23f"),
            ],
        };

        assert!(proof.verify(parse_fp(
            "0100000000000000000000000000000000000000000000000000000000000000"
        )));
    }

    #[test]
    fn rewind() {
        // actual witness data from real note
        // witness at height 3291281 for a note
        // this must be a height greater than the one we rewind to
        let witness = hex::decode("343664116CE838700BBB803556136D4C11B7FF4CAC7CCB3893F8DC20C195470BB483F502015B607DD53B4C8BEBF730047E6B6AAD41E6755112725FC8EFDD4B95FA02D6D3070185A43C44C2CE1F7B95A845A8CCD1B77375AA521BA0F119DA83797A4A5F618B3401E3774C31A10A3622E92FE380FC3C05CDEE0B2B5A4827C7C20E50999E80997E39012970A4E98F6B0DB36380437D391BB9F2A4BDF3AD22C477916AA9A23841E01C3601AC86FE3103502AD0DEE481E0F8B2D5AFEBEEE5E3BA51C3654779179A1FCBDF310153C81D479AD2111644616814F3B1C0614E53EC3435E9873DFB0722421DD5E9020164B671EF44F75A37BC6677AAD2F72062FFFC10B0933941087DD61951FDA4B32401BF05A2E6DB57081595138D60D7C97A1311A67EDDD7919B24FE68E633846B011301AA750059EB14FDCB785A89F3E0B04AE7DD4129A45CFF6DFA748764BDA00FB20D01674A12D6152EC288DD7552F0967492B921965E64B48EE58DD705500D6DB5BE3401A9ABD4B7E6038D5B050090BD039E9EED6ED80AD4B885BF5583B818595B3A8E0E01E4E4C3CBAA0CE3969E51C52E22357F9B7B5131069AACB6BB1D4766D82A86E5390162E4FAB9738EC103E3CAD058F70535BFE04C0BE3912F4327064D3BD0EB7607170168C70212CA93DE252D301E016A8F8AFFDDBE0B94BCDF67E6B8D95CDCDCF2E12601C0D221B2DF44394C6867388C86D996C8C7DF1F89A130E62EE7D9B73721478911015406770A790A86A2E97D198EAEA800387E94FCCF29916527BE06EE5AA54BB6120119AEFA4C6168DE8975F7A49948CB73856D6AD7E25ECA3AB2F8F371A922EE7D2B015FA73F204EB23E5D3C7278F4480300FA07ACABD14FFF9E1E337151D7F594E80701EAC2B89B3F966D833626434DF98D553E000324BBAFB8D6E1FE03B8D7F854CF2A00017C8ECE2B2AB2355D809B58809B21C7A5E95CFC693CD689387F7533EC8749261E01CC2DCAA338B312112DB04B435A706D63244DD435238F0AA1E9E1598D35470810012DCC4273C8A0ED2337ECF7879380A07E7D427C7F9D82E538002BD1442978402C01DAF63DEBF5B40DF902DAE98DADC029F281474D190CDDECEF1B10653248A234150001E2BCA6A8D987D668DEFBA89DC082196A922634ED88E065C669E526BB8815EE1B00000000000069771157B17B027800568FEEB7E2DF1140C3471CDE2D98DC856C8216C228161A").unwrap();
        let (witness, _) = bincode::decode_from_slice::<Witness, _>(&witness, legacy()).unwrap();
        let hasher = OrchardHasher::default();
        let empty_roots = empty_roots(&hasher);

        // tree state at 3240000
        let orchard_tree_state = hex::decode("01c324325bc50e80055a09c2fa1defaf4e35e7a2a4a0bed98aa01ad5a15c51ab3d001f0001ec6a1938e932af981679018acba6febe7fdcf3db817e8d0c680d1b70137bc73f011a42c5862f68c42e3ac35ee7888d5729ec24814d62f9802c856c1842ec98450d000001b8b6e33fbb3a2035e99ca74f23bb0ff777d128b3fa2d7d5a02a1e2902c77a220000001b34d56d339b39e8d33d24900d221bd9fd742b50c18e83f4e477ade4805f65013000189f756fdf90d0faaf9a7933a180be047a23e5a93f7c0b8d5477b506ee1e9a6330000017e172d7cfd8636a30fbc12579ed7e389b310c896e0013965fb1feeb30dd7542601e780c8897bdcc43041b643810bd50f3502b7a199dc763c2270daf494cfde2a3c0001fb82740a3629216088191f9cd359c52a2f35b1c58f6cc905781bd9687b66ad3801eac2b89b3f966d833626434df98d553e000324bbafb8d6e1fe03b8d7f854cf2a00017c8ece2b2ab2355d809b58809b21c7a5e95cfc693cd689387f7533ec8749261e01cc2dcaa338b312112db04b435a706d63244dd435238f0aa1e9e1598d35470810012dcc4273c8a0ed2337ecf7879380a07e7d427c7f9d82e538002bd1442978402c01daf63debf5b40df902dae98dadc029f281474d190cddecef1b10653248a234150001e2bca6a8d987d668defba89dc082196a922634ed88e065c669e526bb8815ee1b000000000000").unwrap();
        let orchard_tree_state =
            CommitmentTreeFrontier::read(orchard_tree_state.as_slice()).unwrap();
        let edge = orchard_tree_state.to_edge(&hasher);
        let edge_auth_path = edge.to_auth_path(&hasher);
        let root0 = edge.root(&hasher);
        // root at the rewound point
        println!("root0: {}", hex::encode(root0));

        let path = witness
            .build_auth_path(&edge_auth_path, &empty_roots)
            .unwrap();
        let root1 = path.root(witness.position, &witness.value, &hasher);

        // new witness should have the same root as the rewound point
        println!("root1: {}", hex::encode(root1));
        assert_eq!(root0, root1);
    }

    // #[tokio::test]
    pub async fn _nullifier_proof() -> Result<()> {
        let nf = hex::decode("F57A9FC4A434CFE6DFB6EE5E930605882A9347D5B6F64BEF90A695C47370BB3B")
            .unwrap();
        let nf: Fp = Fp::from_repr(tiu!(nf)).unwrap();
        let client = PirClient::connect("http://localhost:3000").await?;
        let proof = client.fetch_proof(nf).await?;
        let nf_exclusion_proof = NfExclusion {
            nf_width: proof.width,
            nf_path: MerklePathGeneric::from_parts(proof.low, proof.leaf_pos, proof.path),
        };

        let _nf_exclusion_info = NfExclusionInfo {
            nf_root: proof.root,
            nf_witness: vec![nf_exclusion_proof],
        };
        Ok(())
    }
}
