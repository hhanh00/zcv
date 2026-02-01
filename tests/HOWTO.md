## Update tests

Election definitions are in yaml for convenience
(like `nu7.yml`) but need to be converted to json
and then passed to the GraphQL query `compileElectionDef`.
This is when you specify the election seed phrase
and produce the ElectionProps JSON.
The later is sent to the blockchain via the SetElection
GRPC during bootstrap (see `submit_election.sh for
the example using `grpcurl`[^1]).

Test Ballots are produced in code because they are
blobs obtained by call .write on a Ballot object.

For the sample ballot in `submit_ballots.sh`, you
can take one of the ballots from the database.
Remember that it is the concatenation of the data
and the witnesses. Then call the SubmitVote GRPC
[^2]


[^1]: grpcurl need the string quotes escaped.
[^2]: Binary data must be Base64 encoded
