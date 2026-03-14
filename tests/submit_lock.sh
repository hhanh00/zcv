#!/bin/sh

grpcurl -d '{}' --proto zcvlib/protos/vote.proto --plaintext localhost:9010 cash.z.vote.sdk.rpc.VoteStreamer/Lock
