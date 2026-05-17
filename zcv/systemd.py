"""Systemd service unit file generation and management."""

import logging
import subprocess
from pathlib import Path

logger = logging.getLogger("zcv.systemd")

# Default systemd service name
SERVICE_NAME = "zcv-node"
SYSTEMD_DIR = Path("/etc/systemd/system")


def generate_service_unit(
    dir: Path,
    external_ip: str,
    python_path: str = "/usr/bin/python3",
    description: str = "ZCV Node",
) -> str:
    """Generate a systemd service unit file content.

    Args:
        dir: Working directory for the node.
        external_ip: External IP address (for info only).
        python_path: Path to Python executable.
        description: Service description.

    Returns:
        The service unit file content.
    """
    return f"""[Unit]
Description={description}
After=network.target

[Service]
Type=simple
WorkingDirectory={dir}
ExecStart={python_path} -m zcv.cli daemon --dir {dir}
ExecStop={python_path} -m zcv.cli stop --dir {dir}
Restart=on-failure
RestartSec=10
StandardOutput=append:{dir / "zcv.log"}
StandardError=append:{dir / "zcv.log"}

[Install]
WantedBy=multi-user.target
"""


def install_service(dir: Path, external_ip: str, service_name: str = SERVICE_NAME) -> bool:
    """Install the systemd service unit file.

    Args:
        dir: Working directory for the node.
        external_ip: External IP address.
        service_name: Name for the systemd service.

    Returns:
        True if successful, False otherwise.
    """
    if SYSTEMD_DIR != Path("/etc/systemd/system"):
        logger.warning(f"Non-standard systemd directory: {SYSTEMD_DIR}")

    service_path = SYSTEMD_DIR / f"{service_name}.service"

    # Generate service unit
    import shutil

    python_path = shutil.which("python3") or "/usr/bin/python3"
    unit_content = generate_service_unit(
        dir=dir,
        external_ip=external_ip,
        python_path=python_path,
    )

    try:
        # Write service file
        service_path.write_text(unit_content)

        # Reload systemd
        subprocess.run(["systemctl", "daemon-reload"], check=True)

        # Enable service
        subprocess.run(["systemctl", "enable", service_name], check=True)

        logger.info(f"Service '{service_name}' installed and enabled")
        return True

    except subprocess.CalledProcessError as e:
        logger.error(f"Failed to install service: {e}")
        # Clean up on failure
        if service_path.exists():
            service_path.unlink()
        return False
    except PermissionError:
        logger.error("Permission denied. Try running with sudo.")
        return False


def uninstall_service(service_name: str = SERVICE_NAME) -> bool:
    """Uninstall the systemd service unit file.

    Args:
        service_name: Name of the systemd service.

    Returns:
        True if successful, False otherwise.
    """
    service_path = SYSTEMD_DIR / f"{service_name}.service"

    if not service_path.exists():
        logger.warning(f"Service '{service_name}' is not installed")
        return False

    try:
        # Stop and disable service
        subprocess.run(["systemctl", "stop", service_name], check=False)
        subprocess.run(["systemctl", "disable", service_name], check=False)

        # Remove service file
        service_path.unlink()

        # Reload systemd
        subprocess.run(["systemctl", "daemon-reload"], check=True)

        logger.info(f"Service '{service_name}' uninstalled")
        return True

    except subprocess.CalledProcessError as e:
        logger.error(f"Failed to uninstall service: {e}")
        return False
    except PermissionError:
        logger.error("Permission denied. Try running with sudo.")
        return False


def is_service_installed(service_name: str = SERVICE_NAME) -> bool:
    """Check if the systemd service is installed.

    Args:
        service_name: Name of the systemd service.

    Returns:
        True if the service file exists.
    """
    service_path = SYSTEMD_DIR / f"{service_name}.service"
    return service_path.exists()


def get_service_status(service_name: str = SERVICE_NAME) -> str:
    """Get the status of the systemd service.

    Args:
        service_name: Name of the systemd service.

    Returns:
        The service status string from systemctl.
    """
    try:
        result = subprocess.run(
            ["systemctl", "status", service_name],
            capture_output=True,
            text=True,
        )
        return result.stdout
    except FileNotFoundError:
        return "systemctl not found"
