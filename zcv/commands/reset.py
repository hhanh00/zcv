"""Unsafe reset command."""

import logging

from zcv.config import ZcvConfig
from zcv.process_manager import ProcessManager

logger = logging.getLogger("zcv.reset")


def unsafe_reset(config: ZcvConfig) -> None:
    """Delete all data and reset the node.

    Args:
        config: ZcvConfig instance.
    """
    logger.warning("Performing unsafe reset - all data will be deleted!")

    # Stop processes first
    pm = ProcessManager(config)
    pm.stop_all()

    # Remove vote database
    if config.vote_db.exists():
        config.vote_db.unlink()
        logger.info(f"Removed {config.vote_db}")

    # Run cometbft unsafe-reset-all
    import subprocess

    subprocess.run(
        [str(config.bin_dir / "cometbft"), "unsafe-reset-all", "--home", str(config.cometbft_dir)],
        check=True,
    )

    logger.info("Node reset complete")
