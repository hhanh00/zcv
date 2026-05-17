"""Watchdog for monitoring process health and block progression."""

import logging
import threading
import time
from typing import Optional

import requests

logger = logging.getLogger("zcv.watchdog")


class Watchdog:
    """Monitors process health and block progression."""

    def __init__(self, process_manager, rpc_url: str = "http://localhost:26657"):
        """Initialize the watchdog.

        Args:
            process_manager: ProcessManager instance to monitor.
            rpc_url: CometBFT RPC URL for status checks.
        """
        self.process_manager = process_manager
        self.rpc_url = rpc_url
        self.last_block_height: Optional[int] = None
        self.last_block_time: Optional[float] = None
        self._stop_event = threading.Event()
        self._thread: Optional[threading.Thread] = None

        # Thresholds
        self.block_check_interval = 30  # seconds
        self.block_warning_threshold = 5 * 60  # 5 minutes
        self.block_critical_threshold = 15 * 60  # 15 minutes

    def check_processes(self) -> bool:
        """Check if all processes are running.

        Returns:
            True if all processes are running, False otherwise.
        """
        all_running = self.process_manager.is_all_running()

        if not all_running:
            if not self.process_manager.is_cometbft_running():
                logger.warning("cometbft is not running, restarting...")
                try:
                    self.process_manager.start_cometbft()
                except Exception as e:
                    logger.error(f"Failed to restart cometbft: {e}")

            if not self.process_manager.is_vote_cometbft_running():
                logger.warning("vote-cometbft is not running, restarting...")
                try:
                    self.process_manager.start_vote_cometbft()
                except Exception as e:
                    logger.error(f"Failed to restart vote-cometbft: {e}")

        return all_running

    def check_block_progress(self) -> bool:
        """Check if the node is progressing (block height increasing).

        Returns:
            True if blocks are progressing normally, False if stalled.
        """
        try:
            response = requests.get(f"{self.rpc_url}/status", timeout=5)
            response.raise_for_status()
            data = response.json()

            current_height = int(data["result"]["sync_info"]["latest_block_height"])
            current_time = time.time()

            if self.last_block_height is None:
                self.last_block_height = current_height
                self.last_block_time = current_time
                logger.info(f"Initial block height: {current_height}")
                return True

            if current_height > self.last_block_height:
                # Block progressed, reset timer
                self.last_block_height = current_height
                self.last_block_time = current_time
                logger.debug(f"Block progressed to {current_height}")
                return True

            # No progress, check thresholds
            time_since_last_block = current_time - self.last_block_time

            if time_since_last_block > self.block_critical_threshold:
                logger.error(
                    f"No new blocks for {int(time_since_last_block / 60)} minutes, "
                    f"restarting cometbft..."
                )
                try:
                    self.process_manager.restart_cometbft()
                    self.last_block_time = current_time  # Reset timer
                except Exception as e:
                    logger.error(f"Failed to restart cometbft: {e}")
                return False

            if time_since_last_block > self.block_warning_threshold:
                logger.warning(
                    f"No new blocks for {int(time_since_last_block / 60)} minutes"
                )
                return False

            return True

        except requests.RequestException as e:
            logger.warning(f"Failed to check block progress: {e}")
            return False

    def _run_loop(self) -> None:
        """Main watchdog loop."""
        logger.info("Watchdog started")

        while not self._stop_event.is_set():
            try:
                self.check_processes()
                self.check_block_progress()
            except Exception as e:
                logger.error(f"Watchdog error: {e}")

            # Wait for the interval or stop event
            self._stop_event.wait(self.block_check_interval)

        logger.info("Watchdog stopped")

    def start(self) -> None:
        """Start the watchdog in a background thread."""
        if self._thread is not None and self._thread.is_alive():
            logger.warning("Watchdog is already running")
            return

        self._stop_event.clear()
        self._thread = threading.Thread(target=self._run_loop, daemon=True)
        self._thread.start()

    def stop(self) -> None:
        """Stop the watchdog."""
        if self._thread is None:
            return

        self._stop_event.set()
        self._thread.join(timeout=5)
        self._thread = None

    def is_running(self) -> bool:
        """Check if the watchdog is running.

        Returns:
            True if the watchdog is running.
        """
        return self._thread is not None and self._thread.is_alive()
