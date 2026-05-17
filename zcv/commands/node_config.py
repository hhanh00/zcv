"""Set node configuration command."""

import logging
import subprocess
from pathlib import Path

logger = logging.getLogger("zcv.config")


def set_node_config(config) -> None:
    """Download genesis and configure as a full node.

    Args:
        config: ZcvConfig instance. Requires seed and genesis_url.
    """
    if not config.seed or not config.genesis_url:
        raise ValueError("--seed and --genesis-url are required for set-node-config")

    logger.info("Downloading genesis file...")
    genesis_path = config.cometbft_dir / "config" / "genesis.json"
    download_file(config.genesis_url, genesis_path)

    # Update config.toml
    config_path = config.cometbft_dir / "config" / "config.toml"
    update_config_toml(
        config_path,
        seed=config.seed,
        external_ip=config.external_ip,
    )

    logger.info("Node configured")


def download_file(url: str, dest: Path) -> None:
    """Download a file from URL to destination.

    Args:
        url: Source URL.
        dest: Destination path.
    """
    dest.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["curl", "-L", "-o", str(dest), url], check=True)


def update_config_toml(config_path: Path, seed: str, external_ip: str) -> None:
    """Update the cometbft config.toml file.

    Args:
        config_path: Path to config.toml.
        seed: Seed node address.
        external_ip: External IP address.
    """
    content = config_path.read_text()

    # Update seeds
    content = content.replace('seeds = ""', f'seeds = "{seed}"')

    # Update external address
    content = content.replace(
        'external_address = ""',
        f'external_address = "{external_ip}:26656"',
    )

    # Disable empty blocks
    content = content.replace(
        "create_empty_blocks = true",
        "create_empty_blocks = false",
    )

    config_path.write_text(content)
