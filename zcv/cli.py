"""Main CLI for ZCV node management."""

from pathlib import Path
from typing import Optional

import typer

from zcv.commands.download import download
from zcv.commands.election import lock, promote, set_election
from zcv.commands.node_config import set_node_config
from zcv.commands.reset import unsafe_reset
from zcv.commands.service import get_status, run_daemon, start_background, stop
from zcv.commands.validator import coordinate, show_validators
from zcv.config import ZcvConfig
from zcv.logging import setup_logging
from zcv.systemd import install_service, uninstall_service

app = typer.Typer(
    name="zcv",
    help="ZCV (Zcash Coin Voting) node management CLI",
    add_completion=False,
)


@app.callback()
def main(
    ctx: typer.Context,
    version: bool = typer.Option(
        False,
        "--version",
        help="Show version and exit",
    ),
    dir: Path = typer.Option(
        Path("./zcv-node"),
        "--dir",
        help="Node directory",
    ),
    external_ip: str = typer.Option(
        None,
        "--external-ip",
        help="External IP address",
    ),
    seed: str = typer.Option(
        None,
        "--seed",
        help="Seed node address",
    ),
    genesis_url: str = typer.Option(
        None,
        "--genesis-url",
        help="Genesis file URL",
    ),
    election_json: Path = typer.Option(
        None,
        "--election-json",
        help="Election JSON file path",
        exists=True,
    ),
    log_level: str = typer.Option(
        "INFO",
        "--log-level",
        help="Log level",
    ),
) -> None:
    """ZCV (Zcash Coin Voting) node management CLI."""
    if version:
        from zcv import __version__

        typer.echo(f"zcv-cli {__version__}")
        raise typer.Exit()

    # Store config in context for commands to use
    ctx.ensure_object(dict)
    if external_ip:
        ctx.obj["config"] = ZcvConfig(
            dir=dir,
            external_ip=external_ip,
            seed=seed,
            genesis_url=genesis_url,
            election_json=election_json,
            log_level=log_level,
        )


def get_config(ctx: typer.Context) -> Optional[ZcvConfig]:
    """Get config from context, or prompt for required values."""
    config = ctx.obj.get("config")
    if not config:
        # Prompt for required values
        dir = typer.prompt("Node directory", default="./zcv-node")
        external_ip = typer.prompt("External IP address")

        config = ZcvConfig(
            dir=Path(dir),
            external_ip=external_ip,
            log_level="INFO",
        )
        ctx.obj["config"] = config

    return config


@app.command()
def download(ctx: typer.Context) -> None:
    """Download and install the binaries."""
    config = get_config(ctx)
    setup_logging(config.log_file, config.log_level)
    download(config)


@app.command("set-node-config")
def set_node_config_cmd(ctx: typer.Context) -> None:
    """Download genesis and configure as a full node."""
    config = get_config(ctx)
    setup_logging(config.log_file, config.log_level)
    set_node_config(config)


@app.command()
def coordinate(ctx: typer.Context) -> None:
    """Configure as coordinator and show seed information."""
    config = get_config(ctx)
    from zcv.commands.validator import coordinate as coord

    coord(config)


@app.command()
def daemon(ctx: typer.Context) -> None:
    """Run as daemon (foreground)."""
    config = get_config(ctx)
    run_daemon(config)


@app.command()
def start(ctx: typer.Context) -> None:
    """Start the daemon in background."""
    config = get_config(ctx)
    setup_logging(config.log_file, config.log_level)
    start_background(config)


@app.command()
def stop(ctx: typer.Context) -> None:
    """Stop the daemon."""
    config = get_config(ctx)
    setup_logging(config.log_file, config.log_level)
    stop(config)


@app.command()
def status(ctx: typer.Context) -> None:
    """Show node status."""
    config = get_config(ctx)
    status_info = get_status(config)

    from rich.console import Console
    from rich.table import Table

    console = Console()

    table = Table(title="ZCV Node Status")
    table.add_column("Component", style="cyan")
    table.add_column("Status", style="green")
    table.add_column("Details", style="dim")

    daemon_status = "Running" if status_info["daemon"]["running"] else "Stopped"
    daemon_pid = status_info["daemon"]["pid"] or "N/A"
    table.add_row("Daemon", daemon_status, f"PID: {daemon_pid}")

    for name, info in status_info["processes"].items():
        proc_status = "Running" if info["running"] else "Stopped"
        proc_pid = info["pid"] or "N/A"
        table.add_row(name, proc_status, f"PID: {proc_pid}")

    if status_info["blockchain"]:
        bc = status_info["blockchain"]
        syncing = " (syncing)" if bc["catching_up"] else ""
        table.add_row(
            "Blockchain",
            "Connected" + syncing,
            f"Height: {bc['latest_block_height']}",
        )
    else:
        table.add_row("Blockchain", "Disconnected", "Unable to reach RPC")

    console.print(table)


@app.command("install-service")
def install_service_cmd(
    ctx: typer.Context,
    service_name: str = typer.Option("zcv-node", "--service-name", help="Systemd service name"),
) -> None:
    """Install systemd service."""
    config = get_config(ctx)

    if install_service(config.dir, config.external_ip, service_name):
        typer.echo(f"Service '{service_name}' installed successfully")
        typer.echo(f"Start with: systemctl start {service_name}")
    else:
        typer.echo("Failed to install service", err=True)
        raise typer.Exit(1)


@app.command("uninstall-service")
def uninstall_service_cmd(
    ctx: typer.Context,
    service_name: str = typer.Option("zcv-node", "--service-name", help="Systemd service name"),
) -> None:
    """Uninstall systemd service."""
    if uninstall_service(service_name):
        typer.echo(f"Service '{service_name}' uninstalled successfully")
    else:
        typer.echo("Failed to uninstall service", err=True)
        raise typer.Exit(1)


@app.command("set-election")
def set_election_cmd(ctx: typer.Context) -> None:
    """Set the Election Definition."""
    config = get_config(ctx)
    setup_logging(config.log_file, config.log_level)
    set_election(config)


@app.command()
def lock(ctx: typer.Context) -> None:
    """Lock the Blockchain against further updates."""
    config = get_config(ctx)
    setup_logging(config.log_file, config.log_level)
    lock(config)


@app.command()
def promote(ctx: typer.Context) -> None:
    """Promote to validator."""
    config = get_config(ctx)
    setup_logging(config.log_file, config.log_level)
    promote(config)


@app.command("show-validators")
def show_validators_cmd(ctx: typer.Context) -> None:
    """Show the validator set."""
    config = get_config(ctx)
    show_validators(config)


@app.command("unsafe-reset")
def unsafe_reset_cmd(
    ctx: typer.Context,
    confirm: bool = typer.Option(False, "--confirm", help="Confirm the dangerous operation"),
) -> None:
    """Delete all data and reset the node."""
    config = get_config(ctx)

    if not confirm:
        typer.echo("This is a dangerous operation that will delete all node data.", err=True)
        typer.echo("Use --confirm to proceed.", err=True)
        raise typer.Exit(1)

    setup_logging(config.log_file, config.log_level)
    unsafe_reset(config)


if __name__ == "__main__":
    app()


def main():
    """Entry point for the zcv CLI."""
    app()
