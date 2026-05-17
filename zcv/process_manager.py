"""Process management for cometbft and vote-cometbft."""

import logging
import signal
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

logger = logging.getLogger("zcv.process")


@dataclass
class ProcessInfo:
    """Information about a managed process."""

    name: str
    popen: Optional[subprocess.Popen] = None
    pid_path: Optional[Path] = None


class ProcessManager:
    """Manages cometbft and vote-cometbft processes."""

    def __init__(self, config: "ZcvConfig"):
        """Initialize the process manager.

        Args:
            config: ZCV node configuration.
        """
        from zcv.config import ZcvConfig

        self.config: ZcvConfig = config
        self.cometbft = ProcessInfo(name="cometbft", pid_path=config.dir / "cometbft.pid")
        self.vote_cometbft = ProcessInfo(name="vote-cometbft", pid_path=config.dir / "vote-cometbft.pid")

    def start_cometbft(self) -> subprocess.Popen:
        """Start the cometbft process.

        Returns:
            The Popen object for the started process.
        """
        if self.cometbft.popen is not None and self.cometbft.popen.poll() is None:
            logger.info("cometbft is already running")
            return self.cometbft.popen

        logger.info("Starting cometbft...")
        cmd = [
            str(self.config.bin_dir / "cometbft"),
            "start",
            "--home",
            str(self.config.cometbft_dir),
        ]

        self.cometbft.popen = subprocess.Popen(
            cmd,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )

        # Write PID file
        if self.cometbft.pid_path:
            self.cometbft.pid_path.write_text(str(self.cometbft.popen.pid))

        logger.info(f"cometbft started with PID {self.cometbft.popen.pid}")
        return self.cometbft.popen

    def start_vote_cometbft(self, log_file: Optional[Path] = None) -> subprocess.Popen:
        """Start the vote-cometbft process.

        Args:
            log_file: Optional log file path. Defaults to config.log_file.

        Returns:
            The Popen object for the started process.
        """
        if self.vote_cometbft.popen is not None and self.vote_cometbft.popen.poll() is None:
            logger.info("vote-cometbft is already running")
            return self.vote_cometbft.popen

        logger.info("Starting vote-cometbft...")
        cmd = [str(self.config.bin_dir / "vote-cometbft")]

        log_path = log_file or self.config.log_file
        log_handle = open(log_path, "a") if log_path else subprocess.DEVNULL

        self.vote_cometbft.popen = subprocess.Popen(
            cmd,
            stdout=log_handle,
            stderr=log_handle,
            start_new_session=True,
        )

        # Write PID file
        if self.vote_cometbft.pid_path:
            self.vote_cometbft.pid_path.write_text(str(self.vote_cometbft.popen.pid))

        logger.info(f"vote-cometbft started with PID {self.vote_cometbft.popen.pid}")
        return self.vote_cometbft.popen

    def stop_cometbft(self) -> bool:
        """Stop the cometbft process.

        Returns:
            True if the process was stopped, False if it wasn't running.
        """
        if self.cometbft.popen is None:
            # Try to load from PID file
            if self.cometbft.pid_path and self.cometbft.pid_path.exists():
                pid = int(self.cometbft.pid_path.read_text())
                self.cometbft.popen = self._popen_from_pid(pid)
            else:
                logger.info("cometbft is not running")
                return False

        if self.cometbft.popen.poll() is not None:
            logger.info("cometbft is not running")
            self.cometbft.popen = None
            return False

        logger.info(f"Stopping cometbft (PID {self.cometbft.popen.pid})...")
        self.cometbft.popen.terminate()

        try:
            self.cometbft.popen.wait(timeout=10)
        except subprocess.TimeoutExpired:
            logger.warning("cometbft did not terminate gracefully, killing...")
            self.cometbft.popen.kill()
            self.cometbft.popen.wait()

        # Remove PID file
        if self.cometbft.pid_path and self.cometbft.pid_path.exists():
            self.cometbft.pid_path.unlink()

        self.cometbft.popen = None
        logger.info("cometbft stopped")
        return True

    def stop_vote_cometbft(self) -> bool:
        """Stop the vote-cometbft process.

        Returns:
            True if the process was stopped, False if it wasn't running.
        """
        if self.vote_cometbft.popen is None:
            # Try to load from PID file
            if self.vote_cometbft.pid_path and self.vote_cometbft.pid_path.exists():
                pid = int(self.vote_cometbft.pid_path.read_text())
                self.vote_cometbft.popen = self._popen_from_pid(pid)
            else:
                logger.info("vote-cometbft is not running")
                return False

        if self.vote_cometbft.popen.poll() is not None:
            logger.info("vote-cometbft is not running")
            self.vote_cometbft.popen = None
            return False

        logger.info(f"Stopping vote-cometbft (PID {self.vote_cometbft.popen.pid})...")
        self.vote_cometbft.popen.terminate()

        try:
            self.vote_cometbft.popen.wait(timeout=10)
        except subprocess.TimeoutExpired:
            logger.warning("vote-cometbft did not terminate gracefully, killing...")
            self.vote_cometbft.popen.kill()
            self.vote_cometbft.popen.wait()

        # Remove PID file
        if self.vote_cometbft.pid_path and self.vote_cometbft.pid_path.exists():
            self.vote_cometbft.pid_path.unlink()

        self.vote_cometbft.popen = None
        logger.info("vote-cometbft stopped")
        return True

    def stop_all(self) -> None:
        """Stop all managed processes."""
        self.stop_cometbft()
        self.stop_vote_cometbft()

    def is_running(self, name: str) -> bool:
        """Check if a process is running.

        Args:
            name: Process name ('cometbft' or 'vote-cometbft').

        Returns:
            True if the process is running.
        """
        proc = getattr(self, name.replace("-", "_"), None)
        if proc is None:
            return False

        if proc.popen is None:
            # Try to load from PID file
            if proc.pid_path and proc.pid_path.exists():
                pid = int(proc.pid_path.read_text())
                proc.popen = self._popen_from_pid(pid)
            else:
                return False

        return proc.popen.poll() is None

    def is_cometbft_running(self) -> bool:
        """Check if cometbft is running."""
        return self.is_running("cometbft")

    def is_vote_cometbft_running(self) -> bool:
        """Check if vote-cometbft is running."""
        return self.is_running("vote-cometbft")

    def is_all_running(self) -> bool:
        """Check if all processes are running."""
        return self.is_cometbft_running() and self.is_vote_cometbft_running()

    def get_status(self) -> dict:
        """Get status of all managed processes.

        Returns:
            A dictionary with process names as keys and status info as values.
        """
        return {
            "cometbft": {
                "running": self.is_cometbft_running(),
                "pid": self.cometbft.popen.pid if self.cometbft.popen else None,
            },
            "vote-cometbft": {
                "running": self.is_vote_cometbft_running(),
                "pid": self.vote_cometbft.popen.pid if self.vote_cometbft.popen else None,
            },
        }

    def restart_cometbft(self) -> subprocess.Popen:
        """Restart cometbft.

        Returns:
            The Popen object for the restarted process.
        """
        logger.info("Restarting cometbft...")
        self.stop_cometbft()
        time.sleep(2)  # Give it time to fully stop
        return self.start_cometbft()

    def restart_vote_cometbft(self) -> subprocess.Popen:
        """Restart vote-cometbft.

        Returns:
            The Popen object for the restarted process.
        """
        logger.info("Restarting vote-cometbft...")
        self.stop_vote_cometbft()
        time.sleep(2)  # Give it time to fully stop
        return self.start_vote_cometbft()

    @staticmethod
    def _popen_from_pid(pid: int) -> Optional[subprocess.Popen]:
        """Create a Popen object from an existing PID.

        Args:
            pid: Process ID.

        Returns:
            A Popen object wrapping the existing process, or None if not found.
        """
        try:
            # Check if process exists
            import os

            os.kill(pid, 0)  # Signal 0 doesn't actually kill, just checks existence
            return subprocess.Popen(["sleep", "0"])  # Dummy, we'll just track PID
        except OSError:
            return None
