"""Validator-related commands: show-validators, coordinate."""

import json
import logging
import subprocess

import requests

logger = logging.getLogger("zcv.validator")


def show_validators(config) -> None:
    """Show the validator set.

    Args:
        config: ZcvConfig instance.
    """
    try:
        response = requests.get("http://localhost:26657/validators", timeout=5)
        response.raise_for_status()
        data = response.json()

        # Pretty print
        print(json.dumps(data["result"], indent=2))

    except requests.RequestException as e:
        logger.error(f"Failed to get validators: {e}")


def coordinate(config) -> None:
    """Configure as coordinator and show seed information.

    Args:
        config: ZcvConfig instance.
    """
    print("Configure as seeder")
    print(f"Upload the {config.cometbft_dir / 'config' / 'genesis.json'} file to the cloud")
    print("The seed URL is")

    # Show node ID
    result = subprocess.run(
        [str(config.bin_dir / "cometbft"), "show-node-id", "--home", str(config.cometbft_dir)],
        capture_output=True,
        text=True,
        check=True,
    )

    node_id = result.stdout.strip()
    print(f"{node_id}@{config.external_ip}:26656")
