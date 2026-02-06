# md-docs Test Suite

## Overview

Tests are organized into unit tests (testing individual functions) and integration tests (testing the full pipeline).

Run all tests with:
```bash
cd md-docs/md-docs
source .venv/bin/activate
python -m pytest tests/
```

## Test Structure

```
tests/
├── __init__.py
├── conftest.py              # Shared fixtures (temp dirs, sample markdown, mock config)
├── test_config.py           # Unit tests for config.py
├── test_converter.py        # Unit tests for converter.py (markdown -> LaTeX)
├── test_templates.py        # Unit tests for templates.py (discovery, resolution, modifiers)
├── test_compiler.py         # Unit tests for compiler.py (LaTeX -> PDF)
├── test_cli.py              # Integration tests for CLI commands
└── fixtures/
    ├── sample.md            # Sample markdown with frontmatter
    ├── sample_modifiers.md  # Markdown using various modifiers
    └── expected/            # Expected LaTeX output for comparison
```

## Unit Tests

### test_config.py
- `test_defaults()` — verify DEFAULTS dict has expected keys
- `test_load_config_no_files()` — returns defaults when no config files exist
- `test_load_config_global()` — global config overrides defaults
- `test_load_config_project()` — project config overrides global
- `test_load_config_cli_overrides()` — CLI args override everything
- `test_load_config_ignores_none()` — None values don't override
- `test_ensure_dirs_creates_directories()` — XDG dirs are created

### test_converter.py
- `test_escape_latex()` — special chars are escaped (&, %, $, #, etc.)
- `test_paragraph()` — plain text becomes paragraph
- `test_heading_levels()` — h1-h4 become section/subsection/etc.
- `test_bold_italic()` — **bold** and *italic* render correctly
- `test_links()` — [text](url) becomes \href
- `test_lists_unordered()` — bullet lists render as itemize
- `test_lists_ordered()` — numbered lists render as enumerate
- `test_block_html_modifier()` — HTML comments become LaTeX modifiers
- `test_postprocess_hfill()` — /| becomes \hfill with line break logic
- `test_postprocess_emph_linebreak()` — \emph lines get \\ appended

### test_templates.py
- `test_list_templates_empty()` — returns [] when dir is empty
- `test_list_templates_finds_all()` — finds all template subdirs
- `test_list_brands_empty()` — returns [] when dir is empty
- `test_list_brands_finds_all()` — finds all brand subdirs
- `test_resolve_template_found()` — returns path when template exists
- `test_resolve_template_not_found()` — raises FileNotFoundError
- `test_resolve_template_custom_override()` — custom dir takes precedence
- `test_resolve_brand_found()` — returns path when brand exists
- `test_resolve_brand_not_found()` — raises FileNotFoundError
- `test_get_template_metadata()` — reads metadata.toml correctly
- `test_get_template_metadata_missing()` — falls back to dir name
- `test_load_modifiers()` — loads modifiers.toml
- `test_resolve_modifiers_normal()` — returns latex output
- `test_resolve_modifiers_ignored_remove()` — ignored modifier returns ""
- `test_resolve_modifiers_ignored_newline()` — ignored modifier returns newline
- `test_repo_template_ids()` — returns set of template IDs from cache
- `test_remove_template_success()` — removes repo template
- `test_remove_template_user_added()` — raises ValueError for user template

### test_compiler.py
- `test_check_latex_engine_found()` — returns version string when pdflatex exists
- `test_check_latex_engine_not_found()` — returns None when engine missing
- `test_compile_success()` — generates PDF from valid LaTeX
- `test_compile_failure()` — returns False on invalid LaTeX
- `test_compile_copies_brand()` — brand.tex is copied to temp dir
- `test_compile_copies_fonts()` — fonts/ dir is copied if present

### test_cli.py (Integration)
- `test_cli_no_args_shows_help()` — running with no args prints help
- `test_cli_version()` — --version prints version string
- `test_cli_doctor()` — doctor command runs and returns status
- `test_cli_templates_list_empty()` — templates list with no templates
- `test_cli_templates_list()` — templates list shows installed templates
- `test_cli_brands_list()` — brands list shows installed brands
- `test_cli_convert_missing_file()` — error when input file missing
- `test_cli_convert_missing_template()` — error when template not found
- `test_cli_convert_success()` — full conversion produces PDF
- `test_cli_templates_install_no_git()` — error when git not available
- `test_cli_templates_remove_user_template()` — error when removing user template

## Fixtures (conftest.py)

Key fixtures to implement:

- `tmp_config_dir` — temporary ~/.config/md-docs
- `tmp_data_dir` — temporary ~/.local/share/md-docs with templates/brands/custom subdirs
- `tmp_cache_dir` — temporary ~/.cache/md-docs
- `mock_config` — config dict pointing to temp dirs
- `sample_template` — creates a minimal template in tmp_data_dir
- `sample_brand` — creates a minimal brand in tmp_data_dir
- `sample_markdown` — returns sample markdown content with frontmatter

## Dependencies

Add to pyproject.toml:
```toml
[project.optional-dependencies]
dev = [
    "pytest>=7.0",
    "pytest-mock>=3.10",
]
```
