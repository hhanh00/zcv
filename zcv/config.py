"""Configuration management for ZCV nodes."""

from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class ZcvConfig:
    """Configuration for a ZCV node."""

    dir: Path
    external_ip: str
    seed: Optional[str] = None
    genesis_url: Optional[str] = None
    election_json: Optional[Path] = None
    bin_dir: Path = field(default_factory=lambda: Path("./zcv"))
    log_file: Path = field(default_factory=lambda: Path("zcv.log"))
    log_level: str = "INFO"

    @property
    def cometbft_dir(self) -> Path:
        return self.dir / "cometbft"

    @property
    def protos_dir(self) -> Path:
        return self.dir / "zcv" / "protos"

    @property
    def vote_proto(self) -> Path:
        return self.protos_dir / "vote.proto"

    @property
    def vote_db(self) -> Path:
        return self.dir / "vote.db"
