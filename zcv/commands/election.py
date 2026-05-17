"""Election-related commands: set-election, lock, promote."""

import json
import logging
import subprocess
from pathlib import Path

logger = logging.getLogger("zcv.election")

GRPC_URL = "localhost:9010"
SERVICE = "cash.z.vote.sdk.rpc.VoteStreamer"


def set_election(config) -> None:
    """Set the Election Definition.

    Args:
        config: ZcvConfig instance. Requires election_json.
    """
    if not config.election_json:
        raise ValueError("--election-json is required for set-election")

    logger.info("Configuring the election...")

    # Read election JSON
    election_content = config.election_json.read_text()
    election_data = json.loads(election_content)

    # Build request
    election_req = json.dumps({"election": election_data})

    # Send gRPC request
    result = grpc_call(
        "SetElection",
        election_req,
        proto_path=config.vote_proto,
    )

    if result:
        logger.info("Election configured")


def lock(config) -> None:
    """Lock the Blockchain against further updates.

    Args:
        config: ZcvConfig instance.
    """
    logger.info("Locking blockchain...")

    result = grpc_call(
        "Lock",
        "{}",
        proto_path=config.vote_proto,
    )

    if result:
        logger.info("Blockchain locked")


def promote(config) -> None:
    """Promote the node to validator.

    Args:
        config: ZcvConfig instance.
    """
    logger.info("Promoting to validator...")

    # Get public key
    priv_key_path = config.cometbft_dir / "config" / "priv_validator_key.json"
    priv_key_data = json.loads(priv_key_path.read_text())
    pub_key = priv_key_data["pub_key"]["value"]

    # Build request
    req = json.dumps({"pub_key": pub_key, "power": "10"})

    # Send gRPC request
    result = grpc_call(
        "AddValidator",
        req,
        proto_path=config.vote_proto,
    )

    if result:
        logger.info("Promoted to validator")


def grpc_call(method: str, data: str, proto_path: Path) -> bool:
    """Make a gRPC call using grpcurl.

    Args:
        method: gRPC method name.
        data: JSON request data.
        proto_path: Path to the proto file.

    Returns:
        True if successful, False otherwise.
    """
    cmd = [
        "grpcurl",
        "--plaintext",
        "--proto",
        str(proto_path),
        "-d",
        data,
        GRPC_URL,
        f"{SERVICE}/{method}",
    ]

    try:
        subprocess.run(cmd, check=True, capture_output=True, text=True)
        return True
    except subprocess.CalledProcessError as e:
        logger.error(f"gRPC call failed: {e.stderr}")
        return False
