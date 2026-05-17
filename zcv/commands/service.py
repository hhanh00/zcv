"""Service management commands: daemon, start, stop, status."""

import logging
import signal
import sys
from pathlib import Path

from zcv.config import ZcvConfig
from zcv.logging import setup_logging
from zcv.process_manager import ProcessManager
from zcv.watchdog import Watchdog

logger = logging.getLogger("zcv.service")

# PID file for the daemon
DAEMON_PID_FILE = "zcv-daemon.pid"


def run_daemon(config: ZcvConfig) -> None:
    """Run the daemon in the foreground.

    Args:
        config: ZcvConfig instance.
    """
    setup_logging(config.log_file, config.log_level)

    # Write daemon PID file
    pid_path = config.dir / DAEMON_PID_FILE
    pid_path.write_text(str(Path("/proc").joinpath("self").stat().st_ino))

    logger.info("Starting ZCV daemon...")

    # Create process manager and start processes
    pm = ProcessManager(config)

    # Start cometbft
    try:
        pm.start_cometbft()
    except Exception as e:
        logger.error(f"Failed to start cometbft: {e}")
        sys.exit(1)

    # Start vote-cometbft
    try:
        pm.start_vote_cometbft(log_file=config.log_file)
    except Exception as e:
        logger.error(f"Failed to start vote-cometbft: {e}")
        pm.stop_all()
        sys.exit(1)

    # Start watchdog
    watchdog = Watchdog(pm, rpc_url="http://localhost:26657")
    watchdog.start()

    # Setup signal handlers
    def signal_handler(signum, frame):
        logger.info(f"Received signal {signum}, shutting down...")
        watchdog.stop()
        pm.stop_all()
        if pid_path.exists():
            pid_path.unlink()
        sys.exit(0)

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    # Keep running
    logger.info("ZCV daemon running")
    watchdog._thread.join()


def start_background(config: ZcvConfig) -> None:
    """Start the daemon in the background.

    Args:
        config: ZcvConfig instance.
    """
    pid_path = config.dir / DAEMON_PID_FILE

    if pid_path.exists():
        logger.warning(f"Daemon PID file exists at {pid_path}. Is it already running?")
        return

    # Fork the process
    import os

    pid = os.fork()
    if pid > 0:
        # Parent process
        logger.info(f"Started daemon with PID {pid}")
        pid_path.write_text(str(pid))
        return

    # Child process
    os.setsid()
    os.umask(0)

    # Second fork to prevent zombie process
    pid = os.fork()
    if pid > 0:
        os._exit(0)

    # Redirect stdin/stdout/stderr
    sys.stdin.flush()
    sys.stdout.flush()
    sys.stderr.flush()

    with open("/dev/null", "r") as si:
        os.dup2(si.fileno(), sys.stdin.fileno())
    with open(config.log_file, "a") as so:
        os.dup2(so.fileno(), sys.stdout.fileno())
        os.dup2(so.fileno(), sys.stderr.fileno())

    # Run the daemon
    run_daemon(config)


def stop(config: ZcvConfig) -> None:
    """Stop the daemon.

    Args:
        config: ZcvConfig instance.
    """
    pid_path = config.dir / DAEMON_PID_FILE

    if not pid_path.exists():
        logger.warning("Daemon PID file not found. Is it running?")
        return

    try:
        import os

        pid = int(pid_path.read_text())
        logger.info(f"Stopping daemon (PID {pid})...")
        os.kill(pid, signal.SIGTERM)

        # Wait for process to exit
        import time

        for _ in range(10):
            try:
                os.kill(pid, 0)  # Check if process exists
                time.sleep(1)
            except OSError:
                break
        else:
            logger.warning("Daemon did not stop gracefully, killing...")
            os.kill(pid, signal.SIGKILL)

        pid_path.unlink()
        logger.info("Daemon stopped")

    except (OSError, ValueError) as e:
        logger.error(f"Failed to stop daemon: {e}")


def get_status(config: ZcvConfig) -> dict:
    """Get the status of the node.

    Args:
        config: ZcvConfig instance.

    Returns:
        A dictionary with status information.
    """
    import requests

    status = {
        "daemon": {"running": False, "pid": None},
        "processes": {},
        "blockchain": None,
    }

    # Check daemon
    pid_path = config.dir / DAEMON_PID_FILE
    if pid_path.exists():
        try:
            import os

            pid = int(pid_path.read_text())
            os.kill(pid, 0)  # Check if process exists
            status["daemon"] = {"running": True, "pid": pid}
        except (OSError, ValueError):
            pass

    # Check processes
    pm = ProcessManager(config)
    status["processes"] = pm.get_status()

    # Check blockchain status
    try:
        response = requests.get("http://localhost:26657/status", timeout=2)
        response.raise_for_status()
        data = response.json()

        sync_info = data["result"]["sync_info"]
        status["blockchain"] = {
            "latest_block_height": sync_info["latest_block_height"],
            "latest_block_time": sync_info["latest_block_time"],
            "catching_up": sync_info["catching_up"],
        }
    except requests.RequestException:
        status["blockchain"] = None

    return status
