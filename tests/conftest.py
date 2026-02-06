# conftest.py
# Shared fixtures for md-docs tests.

import sys
from pathlib import Path

import pytest

# Add src to path so we can import modules
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))


@pytest.fixture
def tmp_data_dir(tmp_path):
    """Create a temporary data directory structure."""
    data_dir = tmp_path / "data"
    (data_dir / "templates").mkdir(parents=True)
    (data_dir / "brands").mkdir(parents=True)
    (data_dir / "custom" / "templates").mkdir(parents=True)
    (data_dir / "custom" / "brands").mkdir(parents=True)
    return data_dir


@pytest.fixture
def tmp_config_dir(tmp_path):
    """Create a temporary config directory."""
    config_dir = tmp_path / "config"
    config_dir.mkdir(parents=True)
    return config_dir


@pytest.fixture
def tmp_cache_dir(tmp_path):
    """Create a temporary cache directory."""
    cache_dir = tmp_path / "cache"
    cache_dir.mkdir(parents=True)
    return cache_dir


@pytest.fixture
def mock_config(tmp_data_dir):
    """Return a config dict pointing to temp directories."""
    return {
        "default_template": None,
        "default_brand": None,
        "templates_dir": str(tmp_data_dir / "templates"),
        "brands_dir": str(tmp_data_dir / "brands"),
        "custom_templates_dir": str(tmp_data_dir / "custom" / "templates"),
        "custom_brands_dir": str(tmp_data_dir / "custom" / "brands"),
        "output_dir": None,
        "latex_engine": "pdflatex",
        "author": None,
    }


@pytest.fixture
def sample_template(tmp_data_dir):
    """Create a minimal template and return its path."""
    template_dir = tmp_data_dir / "templates" / "test-template"
    template_dir.mkdir(parents=True)

    # metadata.toml
    (template_dir / "metadata.toml").write_text(
        'name = "Test Template"\n'
        'description = "A test template"\n'
        'default_brand = "test-brand"\n'
    )

    # template.tex - minimal Jinja2/LaTeX template
    (template_dir / "template.tex").write_text(
        r"""\documentclass{article}
\begin{document}
<< content >>
\end{document}
"""
    )

    return template_dir


@pytest.fixture
def sample_brand(tmp_data_dir):
    """Create a minimal brand and return its path."""
    brand_dir = tmp_data_dir / "brands" / "test-brand"
    brand_dir.mkdir(parents=True)

    # metadata.toml
    (brand_dir / "metadata.toml").write_text(
        'name = "Test Brand"\n'
        'description = "A test brand"\n'
    )

    # brand.tex - minimal brand preamble
    (brand_dir / "brand.tex").write_text(
        r"% Test brand preamble"
        "\n"
    )

    return brand_dir


@pytest.fixture
def sample_markdown():
    """Return sample markdown content with frontmatter."""
    return """---
title: Test Document
author: Test Author
---

# Heading One

This is a paragraph with **bold** and *italic* text.

## Heading Two

- Item one
- Item two
- Item three

1. First
2. Second
3. Third

[A link](https://example.com)
"""


@pytest.fixture
def sample_markdown_with_modifiers():
    """Return markdown content using modifiers."""
    return """---
title: Resume
author: Jane Doe
---

# Jane Doe

<!-- COLUMNS_START -->

## Experience

**Software Engineer** /| *2020 - Present*

Built things and did stuff.

<!-- COLUMN_BREAK -->

## Education

**BS Computer Science** /| *2016 - 2020*

Some University
"""
