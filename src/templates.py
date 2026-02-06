# templates.py
# Template and brand discovery and management.
# Finds installed templates and brands in ~/.local/share/md-docs/,
# reads their metadata, lists what's available, and handles
# installation/updates from the templates repo.
# Brands define visual identity (fonts, weights, colors).
# Templates define document layout (resume, paper, letter, etc.).
# Resolves a template + brand pair into paths for the compiler.

import shutil
import subprocess
import tomllib
from pathlib import Path

from config import load_config, CACHE_DIR

TEMPLATES_REPO = "https://github.com/hendemic/md-docs-templates.git"


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
    """Return metadata for all installed templates (repo + custom)."""
    config = config or load_config()
    seen = {}
    # Repo templates first, custom templates override on name collision
    for d in [config["templates_dir"], config["custom_templates_dir"]]:
        for t in _scan_dir(Path(d), get_template_metadata):
            seen[t["id"]] = t
    return sorted(seen.values(), key=lambda m: m["name"])


def list_brands(config: dict | None = None) -> list[dict]:
    """Return metadata for all installed brands (repo + custom)."""
    config = config or load_config()
    seen = {}
    for d in [config["brands_dir"], config["custom_brands_dir"]]:
        for b in _scan_dir(Path(d), get_brand_metadata):
            seen[b["id"]] = b
    return sorted(seen.values(), key=lambda m: m["name"])


def resolve_template(name: str, config: dict | None = None) -> Path:
    """
    Resolve a template name to its directory path.

    Checks custom dir first so user templates override repo templates.
    Raises FileNotFoundError if the template doesn't exist in either location.
    """
    config = config or load_config()
    custom_dir = Path(config["custom_templates_dir"]) / name
    if custom_dir.is_dir():
        return custom_dir
    repo_dir = Path(config["templates_dir"]) / name
    if repo_dir.is_dir():
        return repo_dir
    raise FileNotFoundError(f"Template '{name}' not found")


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

    Checks custom dir first so user brands override repo brands.
    Raises FileNotFoundError if the brand doesn't exist in either location.
    """
    config = config or load_config()
    custom_dir = Path(config["custom_brands_dir"]) / name
    if custom_dir.is_dir():
        return custom_dir
    repo_dir = Path(config["brands_dir"]) / name
    if repo_dir.is_dir():
        return repo_dir
    raise FileNotFoundError(f"Brand '{name}' not found")


# ---------------------------------------------------------------------------
# Template repo management (install / update / remove)
# ---------------------------------------------------------------------------

def _repo_cache_dir() -> Path:
    """Return the cache path for the cloned templates repo."""
    return CACHE_DIR / "md-docs-templates"


def _git(args: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess:
    """Run a git command. Raises RuntimeError on failure."""
    result = subprocess.run(
        ["git"] + args,
        cwd=cwd,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        msg = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"git {' '.join(args)} failed: {msg}")
    return result


def clone_templates_repo() -> Path:
    """
    Clone the templates repo into cache.

    Returns the cache path.
    Raises RuntimeError if the repo is already cloned or git fails.
    """
    cache = _repo_cache_dir()
    if cache.is_dir() and (cache / ".git").is_dir():
        raise RuntimeError(
            "Templates already installed. Use 'md-docs templates update' to update."
        )
    cache.parent.mkdir(parents=True, exist_ok=True)
    _git(["clone", TEMPLATES_REPO, str(cache)])
    return cache


def pull_templates_repo() -> Path:
    """
    Pull latest changes in the cached templates repo.

    Returns the cache path.
    Raises RuntimeError if the repo isn't cloned yet or git fails.
    """
    cache = _repo_cache_dir()
    if not (cache / ".git").is_dir():
        raise RuntimeError(
            "Templates not installed. Run 'md-docs templates install' first."
        )
    _git(["pull"], cwd=cache)
    return cache


def sync_from_cache(config: dict) -> tuple[list[str], list[str]]:
    """
    Copy templates and brands from the cached repo into the data dirs.

    Returns (list of template ids copied, list of brand ids copied).
    Raises RuntimeError if the cache doesn't exist.
    """
    cache = _repo_cache_dir()
    if not cache.is_dir():
        raise RuntimeError("Templates cache not found. Run 'md-docs templates install' first.")

    templates_src = cache / "templates"
    brands_src = cache / "brands"
    templates_dst = Path(config["templates_dir"])
    brands_dst = Path(config["brands_dir"])

    templates_dst.mkdir(parents=True, exist_ok=True)
    brands_dst.mkdir(parents=True, exist_ok=True)

    installed_templates = []
    installed_brands = []

    if templates_src.is_dir():
        for child in sorted(templates_src.iterdir()):
            if child.is_dir() and not child.name.startswith("."):
                dst = templates_dst / child.name
                if dst.exists():
                    shutil.rmtree(dst)
                shutil.copytree(child, dst)
                installed_templates.append(child.name)

    if brands_src.is_dir():
        for child in sorted(brands_src.iterdir()):
            if child.is_dir() and not child.name.startswith("."):
                dst = brands_dst / child.name
                if dst.exists():
                    shutil.rmtree(dst)
                shutil.copytree(child, dst)
                installed_brands.append(child.name)

    return installed_templates, installed_brands


def repo_template_ids() -> set[str]:
    """
    Return the set of template IDs that came from the templates repo.

    Reads from the cached clone. Returns empty set if cache doesn't exist.
    """
    cache = _repo_cache_dir()
    templates_src = cache / "templates"
    if not templates_src.is_dir():
        return set()
    return {
        child.name
        for child in templates_src.iterdir()
        if child.is_dir() and not child.name.startswith(".")
    }


def repo_brand_ids() -> set[str]:
    """
    Return the set of brand IDs that came from the templates repo.

    Reads from the cached clone. Returns empty set if cache doesn't exist.
    """
    cache = _repo_cache_dir()
    brands_src = cache / "brands"
    if not brands_src.is_dir():
        return set()
    return {
        child.name
        for child in brands_src.iterdir()
        if child.is_dir() and not child.name.startswith(".")
    }


def remove_template(name: str, config: dict) -> bool:
    """
    Remove a template from the templates dir.

    Only removes templates that came from the repo (not user-added).
    Returns True if removed, False if not found.
    Raises ValueError if the template is user-added.
    """
    template_dir = Path(config["templates_dir"]) / name
    if not template_dir.is_dir():
        return False
    if name not in repo_template_ids():
        raise ValueError(
            f"Template '{name}' was not installed from the templates repo. "
            "Remove it manually if needed."
        )
    shutil.rmtree(template_dir)
    return True


def remove_brand(name: str, config: dict) -> bool:
    """
    Remove a brand from the brands dir.

    Only removes brands that came from the repo (not user-added).
    Returns True if removed, False if not found.
    Raises ValueError if the brand is user-added.
    """
    brand_dir = Path(config["brands_dir"]) / name
    if not brand_dir.is_dir():
        return False
    if name not in repo_brand_ids():
        raise ValueError(
            f"Brand '{name}' was not installed from the templates repo. "
            "Remove it manually if needed."
        )
    shutil.rmtree(brand_dir)
    return True
