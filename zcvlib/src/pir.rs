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
    use ff::PrimeField;
    use orchard_vote::{MerklePathGeneric, NfExclusion, NfExclusionInfo};
    use pasta_curves::Fp;
    use pir_client::{ImtProofData, PirClient};

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
