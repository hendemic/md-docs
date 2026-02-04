# cli.py
# Entry point and command-line interface for md-docs.
# Handles argument parsing, template selection, frontmatter extraction,
# and orchestrates the conversion pipeline: markdown -> LaTeX -> PDF.

import argparse
import re
import sys
from datetime import datetime
from pathlib import Path

import mistune
import yaml
from jinja2 import Environment, FileSystemLoader

from config import load_config, ensure_dirs
from converter import LatexRenderer, escape_latex, postprocess_latex
from compiler import compile, check_latex_engine
from templates import (
    list_templates,
    list_brands,
    resolve_template,
    resolve_brand,
    get_template_metadata,
    load_modifiers,
    resolve_modifiers,
)

try:
    from simple_term_menu import TerminalMenu
except ImportError:
    TerminalMenu = None


# ---------------------------------------------------------------------------
# Frontmatter
# ---------------------------------------------------------------------------

def parse_frontmatter(content: str) -> tuple[dict, str]:
    """
    Parse YAML frontmatter from markdown content.

    Returns (metadata dict, remaining markdown content).
    """
    pattern = re.compile(r"^---\s*\n(.*?)\n---\s*\n", re.DOTALL)
    match = pattern.match(content)
    if match:
        try:
            metadata = yaml.safe_load(match.group(1)) or {}
        except yaml.YAMLError:
            metadata = {}
        return metadata, content[match.end():]
    return {}, content


# ---------------------------------------------------------------------------
# Interactive TUI selection
# ---------------------------------------------------------------------------

def _select_interactive(items: list[dict], label: str) -> str | None:
    """
    Show an interactive menu for selecting a template or brand.

    Args:
        items: List of metadata dicts (must have 'id' and 'name' keys).
        label: What we're selecting ("template" or "brand").

    Returns:
        Selected item id, or None if cancelled.
    """
    if not items:
        print(f"No {label}s found.", file=sys.stderr)
        return None

    if len(items) == 1:
        print(f"Using {label}: {items[0]['name']}")
        return items[0]["id"]

    if TerminalMenu is None:
        # No TUI library — fall back to first item
        print(f"Using default {label}: {items[0]['name']}")
        print(f"  (Tip: specify with flag or install simple-term-menu for interactive selection)")
        return items[0]["id"]

    menu_items = [item["name"] for item in items]
    menu = TerminalMenu(menu_items, title=f"Select a {label}:")
    idx = menu.show()
    if idx is None:
        return None
    return items[idx]["id"]


# ---------------------------------------------------------------------------
# Conversion pipeline
# ---------------------------------------------------------------------------

def markdown_to_latex(content: str, modifiers: dict) -> str:
    """Convert markdown content to LaTeX via the custom renderer."""
    renderer = LatexRenderer(modifiers=modifiers)
    md = mistune.create_markdown(
        renderer=renderer,
        plugins=["table", "strikethrough"],
    )
    latex = md(content)
    return postprocess_latex(latex, modifiers)


def render_template(
    template_dir: Path,
    metadata: dict,
    latex_content: str,
) -> str:
    """
    Render a Jinja2/LaTeX template with converted content and metadata.

    Uses custom delimiters to avoid clashing with LaTeX syntax:
      <% %> blocks, << >> variables, <# #> comments.
    """
    template_file = template_dir / "template.tex"
    if not template_file.is_file():
        raise FileNotFoundError(
            f"No template.tex found in {template_dir}"
        )

    env = Environment(
        loader=FileSystemLoader(str(template_dir)),
        block_start_string="<%",
        block_end_string="%>",
        variable_start_string="<<",
        variable_end_string=">>",
        comment_start_string="<#",
        comment_end_string="#>",
    )

    template = env.get_template("template.tex")

    # Split content at %%COLUMNS_START%% if present
    split_marker = "%%COLUMNS_START%%"
    if split_marker in latex_content:
        header, body = latex_content.split(split_marker, 1)
    else:
        header = ""
        body = latex_content

    context = {
        "title": escape_latex(metadata.get("title", "Untitled Document")),
        "subtitle": escape_latex(metadata.get("subtitle", "")),
        "author": escape_latex(metadata.get("author", "")),
        "date": escape_latex(
            metadata.get("date", datetime.now().strftime("%B %d, %Y"))
        ),
        "year": metadata.get("year", datetime.now().strftime("%Y")),
        "content": latex_content,
        "header": header.strip(),
        "body": body.strip(),
    }

    # Pass through any extra frontmatter keys (escaped)
    for key, value in metadata.items():
        if key not in context and isinstance(value, str):
            context[key] = escape_latex(value)

    return template.render(**context)


# ---------------------------------------------------------------------------
# Subcommand handlers
# ---------------------------------------------------------------------------

def cmd_convert(args, config: dict) -> int:
    """Handle the 'convert' subcommand."""
    input_path = Path(args.input)
    if not input_path.is_file():
        print(f"Error: file not found: {input_path}", file=sys.stderr)
        return 1

    # Resolve output path
    if args.output:
        output_path = Path(args.output)
    elif args.directory:
        out_dir = Path(args.directory)
        out_dir.mkdir(parents=True, exist_ok=True)
        output_path = out_dir / input_path.with_suffix(".pdf").name
    else:
        output_path = input_path.with_suffix(".pdf")

    # Resolve template
    template_name = args.template or config.get("default_template")
    if not template_name:
        template_name = _select_interactive(
            list_templates(config), "template"
        )
        if not template_name:
            print("Cancelled.", file=sys.stderr)
            return 1

    try:
        template_dir = resolve_template(template_name, config)
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    # Read template metadata (used for brand default + modifier ignore list)
    tmpl_meta = get_template_metadata(template_dir)

    # Resolve brand
    brand_name = args.brand or config.get("default_brand")
    if not brand_name:
        brand_name = tmpl_meta.get("default_brand")
    if not brand_name:
        brand_name = _select_interactive(list_brands(config), "brand")
        if not brand_name:
            print("Cancelled.", file=sys.stderr)
            return 1

    try:
        brand_dir = resolve_brand(brand_name, config)
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    # Load and resolve modifiers for this template
    all_modifiers = load_modifiers()
    modifiers = resolve_modifiers(all_modifiers, tmpl_meta.get("ignore", []))

    # Read and parse markdown
    content = input_path.read_text(encoding="utf-8")
    metadata, markdown_body = parse_frontmatter(content)

    # Inject config author as fallback
    if not metadata.get("author") and config.get("author"):
        metadata["author"] = config["author"]

    # Convert markdown -> LaTeX
    latex_content = markdown_to_latex(markdown_body, modifiers)

    # Render template
    try:
        full_latex = render_template(template_dir, metadata, latex_content)
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    # Compile LaTeX -> PDF
    engine = config.get("latex_engine", "pdflatex")
    if compile(full_latex, output_path, template_dir, brand_dir, engine=engine):
        print(f"Generated: {output_path}")
        return 0
    return 1


def cmd_templates_list(args, config: dict) -> int:
    """Handle 'templates list' subcommand."""
    templates = list_templates(config)
    if not templates:
        print("No templates installed.")
        return 0
    for t in templates:
        desc = f" — {t['description']}" if t.get("description") else ""
        print(f"  {t['id']}: {t['name']}{desc}")
    return 0


def cmd_brands_list(args, config: dict) -> int:
    """Handle 'brands list' subcommand."""
    brands = list_brands(config)
    if not brands:
        print("No brands installed.")
        return 0
    for b in brands:
        desc = f" — {b['description']}" if b.get("description") else ""
        print(f"  {b['id']}: {b['name']}{desc}")
    return 0


def cmd_doctor(args, config: dict) -> int:
    """Handle the 'doctor' subcommand."""
    ok = True

    # Check LaTeX engine
    engine = config.get("latex_engine", "pdflatex")
    version = check_latex_engine(engine)
    if version:
        print(f"{engine}: {version}")
    else:
        print(f"{engine}: NOT FOUND")
        ok = False

    # Check templates directory
    templates_dir = Path(config["templates_dir"])
    count = len(list_templates(config))
    print(f"Templates dir: {templates_dir} ({count} installed)")

    # Check brands directory
    brands_dir = Path(config["brands_dir"])
    count = len(list_brands(config))
    print(f"Brands dir: {brands_dir} ({count} installed)")

    return 0 if ok else 1


# ---------------------------------------------------------------------------
# Argument parser
# ---------------------------------------------------------------------------

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="md-docs",
        description="Generate professional documents from Markdown.",
    )
    subparsers = parser.add_subparsers(dest="command")

    # --- convert ---
    convert_parser = subparsers.add_parser(
        "convert", help="Convert a Markdown file to PDF"
    )
    convert_parser.add_argument("input", help="Input Markdown file")
    convert_parser.add_argument(
        "-t", "--template", default=None, help="Template name"
    )
    convert_parser.add_argument(
        "-b", "--brand", default=None, help="Brand name"
    )
    output_group = convert_parser.add_mutually_exclusive_group()
    output_group.add_argument(
        "-o", "--output", help="Output PDF path"
    )
    output_group.add_argument(
        "-d", "--directory", help="Output directory"
    )

    # --- templates list ---
    templates_parser = subparsers.add_parser(
        "templates", help="Manage templates"
    )
    templates_sub = templates_parser.add_subparsers(dest="templates_cmd")
    templates_sub.add_parser("list", help="List installed templates")

    # --- brands list ---
    brands_parser = subparsers.add_parser(
        "brands", help="Manage brands"
    )
    brands_sub = brands_parser.add_subparsers(dest="brands_cmd")
    brands_sub.add_parser("list", help="List installed brands")

    # --- doctor ---
    subparsers.add_parser(
        "doctor", help="Check system dependencies and configuration"
    )

    return parser


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> int:
    ensure_dirs()
    parser = build_parser()
    args = parser.parse_args()
    config = load_config()

    if args.command == "convert":
        return cmd_convert(args, config)
    elif args.command == "templates":
        if getattr(args, "templates_cmd", None) == "list":
            return cmd_templates_list(args, config)
        parser.parse_args(["templates", "--help"])
    elif args.command == "brands":
        if getattr(args, "brands_cmd", None) == "list":
            return cmd_brands_list(args, config)
        parser.parse_args(["brands", "--help"])
    elif args.command == "doctor":
        return cmd_doctor(args, config)
    else:
        parser.print_help()
        return 0

    return 0


if __name__ == "__main__":
    sys.exit(main())
