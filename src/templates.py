# templates.py
# Template and brand discovery and management.
# Finds installed templates and brands in ~/.local/share/md-docs/,
# reads their metadata, lists what's available, and handles
# installation/updates from the templates repo.
# Brands define visual identity (fonts, weights, colors).
# Templates define document layout (resume, paper, letter, etc.).
# Resolves a template + brand pair into paths for the compiler.

import tomllib
from pathlib import Path

from config import load_config


def _read_metadata_toml(path: Path) -> dict:
    """Read a metadata.toml file, returning empty dict if missing."""
    toml_path = path / "metadata.toml"
    if not toml_path.is_file():
        return {}
    try:
        with open(toml_path, "rb") as f:
            return tomllib.load(f)
    except (tomllib.TOMLDecodeError, OSError):
        return {}


def get_template_metadata(template_path: Path) -> dict:
    """
    Read metadata from a template directory.

    Returns dict with: id, name, description, default_brand, ignore.
    Falls back to directory name for display name.
    """
    raw = _read_metadata_toml(template_path)
    return {
        "id": template_path.name,
        "name": raw.get("name", template_path.name.replace("_", " ").title()),
        "description": raw.get("description"),
        "default_brand": raw.get("default_brand"),
        "ignore": raw.get("ignore", []),
    }


def get_brand_metadata(brand_path: Path) -> dict:
    """
    Read metadata from a brand directory.

    Returns dict with: id, name, description.
    Falls back to directory name for display name.
    """
    raw = _read_metadata_toml(brand_path)
    return {
        "id": brand_path.name,
        "name": raw.get("name", brand_path.name.replace("_", " ").title()),
        "description": raw.get("description"),
    }


def _scan_dir(base_dir: Path, metadata_fn) -> list[dict]:
    """Scan a directory for subdirectories and read their metadata."""
    if not base_dir.is_dir():
        return []
    results = []
    for child in base_dir.iterdir():
        if child.is_dir() and not child.name.startswith("."):
            results.append(metadata_fn(child))
    return sorted(results, key=lambda m: m["name"])


def list_templates(config: dict | None = None) -> list[dict]:
    """Return metadata for all installed templates."""
    config = config or load_config()
    templates_dir = Path(config["templates_dir"])
    return _scan_dir(templates_dir, get_template_metadata)


def list_brands(config: dict | None = None) -> list[dict]:
    """Return metadata for all installed brands."""
    config = config or load_config()
    brands_dir = Path(config["brands_dir"])
    return _scan_dir(brands_dir, get_brand_metadata)


def resolve_template(name: str, config: dict | None = None) -> Path:
    """
    Resolve a template name to its directory path.

    Raises FileNotFoundError if the template doesn't exist.
    """
    config = config or load_config()
    template_dir = Path(config["templates_dir"]) / name
    if not template_dir.is_dir():
        raise FileNotFoundError(f"Template '{name}' not found in {config['templates_dir']}")
    return template_dir


def load_modifiers() -> dict:
    """
    Load modifiers.toml from the app repo root.

    Returns the raw dict of modifier definitions keyed by modifier id.
    """
    # modifiers.toml lives at the app repo root (one level up from src/)
    app_root = Path(__file__).parent.parent
    modifiers_path = app_root / "modifiers.toml"
    if not modifiers_path.is_file():
        return {}
    try:
        with open(modifiers_path, "rb") as f:
            return tomllib.load(f)
    except (tomllib.TOMLDecodeError, OSError):
        return {}


def resolve_modifiers(modifiers: dict, ignore_list: list[str]) -> dict:
    """
    Build the effective modifier map for a template.

    For each modifier, if it's in the template's ignore list, substitute
    its on_ignore behavior. Otherwise use its normal latex output.

    Args:
        modifiers: Full modifier definitions from modifiers.toml.
        ignore_list: List of modifier ids the template wants to ignore.

    Returns:
        Dict mapping modifier id to a dict with:
            marker, latex, type
        where latex reflects the on_ignore replacement if ignored.
    """
    resolved = {}
    ignore_set = set(ignore_list)
    for mod_id, mod_def in modifiers.items():
        marker = mod_def.get("marker", "")
        mod_type = mod_def.get("type", "block")

        if mod_id in ignore_set:
            on_ignore = mod_def.get("on_ignore", "remove")
            if on_ignore == "remove":
                latex = ""
            elif on_ignore == "newline":
                latex = " \\\\\n" if mod_type == "inline" else "\n\n"
            else:  # "keep"
                latex = None  # signal to leave marker as-is
        else:
            latex = mod_def.get("latex", "")

        resolved[mod_id] = {
            "marker": marker,
            "latex": latex,
            "type": mod_type,
        }
    return resolved


def resolve_brand(name: str, config: dict | None = None) -> Path:
    """
    Resolve a brand name to its directory path.

    Raises FileNotFoundError if the brand doesn't exist.
    """
    config = config or load_config()
    brand_dir = Path(config["brands_dir"]) / name
    if not brand_dir.is_dir():
        raise FileNotFoundError(f"Brand '{name}' not found in {config['brands_dir']}")
    return brand_dir
