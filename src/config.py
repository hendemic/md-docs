# config.py
# Handles layered configuration resolution.
# Loads global config from ~/.config/md-docs/config.toml,
# merges with project-level .md-docs.toml if present,
# and applies CLI argument overrides on top.

import os
import tomllib
from pathlib import Path


def _xdg_path(env_var: str, fallback: str) -> Path:
    """Resolve an XDG base directory, respecting env overrides."""
    return Path(os.environ.get(env_var, Path.home() / fallback))


CONFIG_DIR = _xdg_path("XDG_CONFIG_HOME", ".config") / "md-docs"
DATA_DIR = _xdg_path("XDG_DATA_HOME", ".local/share") / "md-docs"
CACHE_DIR = _xdg_path("XDG_CACHE_HOME", ".cache") / "md-docs"

DEFAULTS = {
    "default_template": None,
    "default_brand": None,
    "templates_dir": str(DATA_DIR / "templates"),
    "brands_dir": str(DATA_DIR / "brands"),
    "output_dir": None,
    "latex_engine": "pdflatex",
    "author": None,
}


def _load_toml(path: Path) -> dict:
    """Load a TOML file, returning empty dict if missing or invalid."""
    if not path.is_file():
        return {}
    try:
        with open(path, "rb") as f:
            return tomllib.load(f)
    except (tomllib.TOMLDecodeError, OSError):
        return {}


def load_config(cli_overrides: dict | None = None) -> dict:
    """
    Build final config by layering: defaults <- global <- project <- CLI.

    Args:
        cli_overrides: Dict of values from CLI flags. None values are ignored.

    Returns:
        Resolved config dict.
    """
    config = dict(DEFAULTS)

    # Layer 1: global config
    global_config = _load_toml(CONFIG_DIR / "config.toml")
    for key, value in global_config.items():
        if key in config and value is not None:
            config[key] = value

    # Layer 2: project-level config
    project_config = _load_toml(Path.cwd() / ".md-docs.toml")
    for key, value in project_config.items():
        if key in config and value is not None:
            config[key] = value

    # Layer 3: CLI overrides
    if cli_overrides:
        for key, value in cli_overrides.items():
            if key in config and value is not None:
                config[key] = value

    return config


def ensure_dirs() -> None:
    """Create XDG directories if they don't exist."""
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    Path(DEFAULTS["templates_dir"]).mkdir(parents=True, exist_ok=True)
    Path(DEFAULTS["brands_dir"]).mkdir(parents=True, exist_ok=True)
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
