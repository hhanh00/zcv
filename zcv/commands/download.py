"""Download and install binaries command."""

import logging
import subprocess
from pathlib import Path

logger = logging.getLogger("zcv.download")

# Version to download
VERSION = "zcvlib-v0.6.0"
BASE_URL = f"https://github.com/hhanh00/zcv/releases/download/{VERSION}"


def download(config) -> None:
    """Download and install the required binaries.

    Args:
        config: ZcvConfig instance.
    """
    logger.info("Installing binaries...")

    # Create directories
    config.bin_dir.mkdir(parents=True, exist_ok=True)
    config.protos_dir.mkdir(parents=True, exist_ok=True)

    # Copy cometbft from GOPATH
    gopath_bin = Path.home() / "go" / "bin" / "cometbft"
    if gopath_bin.exists():
        import shutil

        shutil.copy(gopath_bin, config.bin_dir / "cometbft")
        logger.info("Copied cometbft from GOPATH")
    else:
        logger.warning(f"cometbft not found at {gopath_bin}")

    # Download vote-cometbft
    vote_cometbft_path = config.bin_dir / "vote-cometbft"
    download_file(f"{BASE_URL}/vote-cometbft", vote_cometbft_path)
    vote_cometbft_path.chmod(0o755)
    logger.info("Downloaded vote-cometbft")

    # Download vote.proto
    proto_url = f"https://raw.githubusercontent.com/hhanh00/zcv/refs/tags/{VERSION}/zcvlib/protos/vote.proto"
    download_file(proto_url, config.vote_proto)
    logger.info("Downloaded vote.proto")

    # Initialize cometbft
    subprocess.run(
        [str(config.bin_dir / "cometbft"), "init", "--home", str(config.cometbft_dir)],
        check=True,
    )
    logger.info("Initialized cometbft")


def download_file(url: str, dest: Path) -> None:
    """Download a file from URL to destination.

    Args:
        url: Source URL.
        dest: Destination path.
    """
    subprocess.run(["curl", "-L", "-o", str(dest), url], check=True)
