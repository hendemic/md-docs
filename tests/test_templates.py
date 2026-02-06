# test_templates.py
# Unit tests for templates.py

import pytest

from templates import (
    list_templates,
    list_brands,
    resolve_template,
    resolve_brand,
    get_template_metadata,
    get_brand_metadata,
    load_modifiers,
    resolve_modifiers,
)


class TestListTemplates:
    """Tests for list_templates()."""

    def test_returns_empty_list_when_dir_empty(self, mock_config):
        result = list_templates(mock_config)
        assert result == []

    def test_finds_all_templates(self, mock_config, sample_template):
        result = list_templates(mock_config)
        assert len(result) == 1
        assert result[0]["id"] == "test-template"
        assert result[0]["name"] == "Test Template"

    def test_ignores_hidden_directories(self, mock_config, tmp_data_dir):
        # Create a hidden directory
        hidden_dir = tmp_data_dir / "templates" / ".hidden"
        hidden_dir.mkdir()
        (hidden_dir / "metadata.toml").write_text('name = "Hidden"\n')

        result = list_templates(mock_config)
        assert len(result) == 0

    def test_multiple_templates_sorted_by_name(self, mock_config, tmp_data_dir):
        # Create templates with names that sort differently than ids
        for name, display in [("z-template", "Alpha"), ("a-template", "Zeta")]:
            d = tmp_data_dir / "templates" / name
            d.mkdir()
            (d / "metadata.toml").write_text(f'name = "{display}"\n')

        result = list_templates(mock_config)
        assert len(result) == 2
        assert result[0]["name"] == "Alpha"
        assert result[1]["name"] == "Zeta"


class TestListBrands:
    """Tests for list_brands()."""

    def test_returns_empty_list_when_dir_empty(self, mock_config):
        result = list_brands(mock_config)
        assert result == []

    def test_finds_all_brands(self, mock_config, sample_brand):
        result = list_brands(mock_config)
        assert len(result) == 1
        assert result[0]["id"] == "test-brand"
        assert result[0]["name"] == "Test Brand"


class TestResolveTemplate:
    """Tests for resolve_template()."""

    def test_returns_path_when_found(self, mock_config, sample_template):
        result = resolve_template("test-template", mock_config)
        assert result == sample_template
        assert result.is_dir()

    def test_raises_when_not_found(self, mock_config):
        with pytest.raises(FileNotFoundError):
            resolve_template("nonexistent", mock_config)


class TestResolveBrand:
    """Tests for resolve_brand()."""

    def test_returns_path_when_found(self, mock_config, sample_brand):
        result = resolve_brand("test-brand", mock_config)
        assert result == sample_brand
        assert result.is_dir()

    def test_raises_when_not_found(self, mock_config):
        with pytest.raises(FileNotFoundError):
            resolve_brand("nonexistent", mock_config)


class TestGetTemplateMetadata:
    """Tests for get_template_metadata()."""

    def test_reads_metadata_toml(self, sample_template):
        result = get_template_metadata(sample_template)
        assert result["id"] == "test-template"
        assert result["name"] == "Test Template"
        assert result["description"] == "A test template"
        assert result["default_brand"] == "test-brand"

    def test_fallback_when_no_metadata(self, tmp_path):
        # Create template dir without metadata.toml
        template_dir = tmp_path / "no_metadata"
        template_dir.mkdir()

        result = get_template_metadata(template_dir)
        assert result["id"] == "no_metadata"
        assert result["name"] == "No Metadata"  # Title-cased from dir name, underscores to spaces
        assert result["description"] is None
        assert result["default_brand"] is None

    def test_returns_ignore_list(self, tmp_path):
        template_dir = tmp_path / "with-ignore"
        template_dir.mkdir()
        (template_dir / "metadata.toml").write_text(
            'name = "Test"\n'
            'ignore = ["date_separator", "column_break"]\n'
        )

        result = get_template_metadata(template_dir)
        assert result["ignore"] == ["date_separator", "column_break"]


class TestGetBrandMetadata:
    """Tests for get_brand_metadata()."""

    def test_reads_metadata_toml(self, sample_brand):
        result = get_brand_metadata(sample_brand)
        assert result["id"] == "test-brand"
        assert result["name"] == "Test Brand"
        assert result["description"] == "A test brand"

    def test_fallback_when_no_metadata(self, tmp_path):
        brand_dir = tmp_path / "plain_brand"
        brand_dir.mkdir()

        result = get_brand_metadata(brand_dir)
        assert result["id"] == "plain_brand"
        assert result["name"] == "Plain Brand"  # Title-cased, underscore to space


class TestLoadModifiers:
    """Tests for load_modifiers()."""

    def test_loads_modifiers_toml(self):
        # This tests against the actual modifiers.toml in the repo
        result = load_modifiers()
        assert "date_separator" in result
        assert "column_break" in result
        assert "columns_start" in result

    def test_modifier_has_expected_fields(self):
        result = load_modifiers()
        date_sep = result["date_separator"]
        assert "marker" in date_sep
        assert "latex" in date_sep
        assert "type" in date_sep
        assert "on_ignore" in date_sep


class TestResolveModifiers:
    """Tests for resolve_modifiers()."""

    @pytest.fixture
    def sample_modifiers(self):
        return {
            "date_separator": {
                "marker": " /| ",
                "latex": r" \hfill ",
                "type": "inline",
                "on_ignore": "newline",
            },
            "column_break": {
                "marker": "<!-- COLUMN_BREAK -->",
                "latex": r"\switchcolumn",
                "type": "block",
                "on_ignore": "remove",
            },
        }

    def test_returns_latex_when_not_ignored(self, sample_modifiers):
        result = resolve_modifiers(sample_modifiers, [])
        assert result["date_separator"]["latex"] == r" \hfill "
        assert result["column_break"]["latex"] == r"\switchcolumn"

    def test_ignored_remove_returns_empty(self, sample_modifiers):
        result = resolve_modifiers(sample_modifiers, ["column_break"])
        assert result["column_break"]["latex"] == ""

    def test_ignored_newline_returns_newline(self, sample_modifiers):
        result = resolve_modifiers(sample_modifiers, ["date_separator"])
        # Inline modifier with newline on_ignore
        assert r"\\" in result["date_separator"]["latex"]

    def test_preserves_marker_and_type(self, sample_modifiers):
        result = resolve_modifiers(sample_modifiers, ["column_break"])
        assert result["column_break"]["marker"] == "<!-- COLUMN_BREAK -->"
        assert result["column_break"]["type"] == "block"
