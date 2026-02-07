for i in {1..10000}; do
    gq http://127.0.0.1:8000/graphql \
    -q 'mutation vote ($hash: String!) {vote(idAccount: 2 amount: "0.1" hash: $hash idxQuestion: 1 voteContent: "010201")}' \
    -v hash="059f7f47936cbc080942035dded3f16d0e08b29347e08239dbba61c199de62f7"

    gq http://127.0.0.1:8000/graphql \
    -q 'mutation scan($hash: String!) {scanBallots(hash: $hash idAccount: 2)}' \
    -v hash="059f7f47936cbc080942035dded3f16d0e08b29347e08239dbba61c199de62f7"

    sleep 1
done
