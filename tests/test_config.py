# test_config.py
# Unit tests for config.py

import os
from pathlib import Path
from unittest.mock import patch

import pytest

from config import DEFAULTS, load_config, ensure_dirs, CONFIG_DIR, DATA_DIR, CACHE_DIR


class TestDefaults:
    """Tests for the DEFAULTS dict."""

    def test_defaults_has_expected_keys(self):
        expected_keys = {
            "default_template",
            "default_brand",
            "templates_dir",
            "brands_dir",
            "custom_templates_dir",
            "custom_brands_dir",
            "output_dir",
            "latex_engine",
            "author",
        }
        assert set(DEFAULTS.keys()) == expected_keys

    def test_defaults_latex_engine_is_pdflatex(self):
        assert DEFAULTS["latex_engine"] == "pdflatex"

    def test_defaults_templates_dir_is_under_data_dir(self):
        assert "md-docs" in DEFAULTS["templates_dir"]
        assert "templates" in DEFAULTS["templates_dir"]

    def test_defaults_brands_dir_is_under_data_dir(self):
        assert "md-docs" in DEFAULTS["brands_dir"]
        assert "brands" in DEFAULTS["brands_dir"]


class TestLoadConfig:
    """Tests for load_config()."""

    def test_load_config_returns_defaults_when_no_files(self, tmp_path, monkeypatch):
        # Point to empty directories so no config files are found
        monkeypatch.chdir(tmp_path)
        with patch("config.CONFIG_DIR", tmp_path / "config"):
            config = load_config()
            assert config["latex_engine"] == "pdflatex"
            assert config["default_template"] is None

    def test_load_config_global_overrides_defaults(self, tmp_path, monkeypatch):
        config_dir = tmp_path / "config"
        config_dir.mkdir()
        (config_dir / "config.toml").write_text('latex_engine = "xelatex"\n')

        monkeypatch.chdir(tmp_path)
        with patch("config.CONFIG_DIR", config_dir):
            config = load_config()
            assert config["latex_engine"] == "xelatex"

    def test_load_config_project_overrides_global(self, tmp_path, monkeypatch):
        config_dir = tmp_path / "config"
        config_dir.mkdir()
        (config_dir / "config.toml").write_text('latex_engine = "xelatex"\n')

        project_dir = tmp_path / "project"
        project_dir.mkdir()
        (project_dir / ".md-docs.toml").write_text('latex_engine = "lualatex"\n')

        monkeypatch.chdir(project_dir)
        with patch("config.CONFIG_DIR", config_dir):
            config = load_config()
            assert config["latex_engine"] == "lualatex"

    def test_load_config_cli_overrides_all(self, tmp_path, monkeypatch):
        config_dir = tmp_path / "config"
        config_dir.mkdir()
        (config_dir / "config.toml").write_text('latex_engine = "xelatex"\n')

        monkeypatch.chdir(tmp_path)
        with patch("config.CONFIG_DIR", config_dir):
            config = load_config(cli_overrides={"latex_engine": "pdflatex"})
            assert config["latex_engine"] == "pdflatex"

    def test_load_config_cli_none_values_ignored(self, tmp_path, monkeypatch):
        config_dir = tmp_path / "config"
        config_dir.mkdir()
        (config_dir / "config.toml").write_text('latex_engine = "xelatex"\n')

        monkeypatch.chdir(tmp_path)
        with patch("config.CONFIG_DIR", config_dir):
            config = load_config(cli_overrides={"latex_engine": None})
            assert config["latex_engine"] == "xelatex"

    def test_load_config_unknown_keys_ignored(self, tmp_path, monkeypatch):
        config_dir = tmp_path / "config"
        config_dir.mkdir()
        (config_dir / "config.toml").write_text('unknown_key = "value"\n')

        monkeypatch.chdir(tmp_path)
        with patch("config.CONFIG_DIR", config_dir):
            config = load_config()
            assert "unknown_key" not in config


class TestEnsureDirs:
    """Tests for ensure_dirs()."""

    def test_ensure_dirs_creates_directories(self, tmp_path, monkeypatch):
        with patch("config.CONFIG_DIR", tmp_path / "config" / "md-docs"), \
             patch("config.CACHE_DIR", tmp_path / "cache" / "md-docs"), \
             patch("config.DEFAULTS", {
                 **DEFAULTS,
                 "templates_dir": str(tmp_path / "data" / "md-docs" / "templates"),
                 "brands_dir": str(tmp_path / "data" / "md-docs" / "brands"),
                 "custom_templates_dir": str(tmp_path / "data" / "md-docs" / "custom" / "templates"),
                 "custom_brands_dir": str(tmp_path / "data" / "md-docs" / "custom" / "brands"),
             }):
            ensure_dirs()

            assert (tmp_path / "config" / "md-docs").is_dir()
            assert (tmp_path / "cache" / "md-docs").is_dir()
            assert (tmp_path / "data" / "md-docs" / "templates").is_dir()
            assert (tmp_path / "data" / "md-docs" / "brands").is_dir()
            assert (tmp_path / "data" / "md-docs" / "custom" / "templates").is_dir()
            assert (tmp_path / "data" / "md-docs" / "custom" / "brands").is_dir()
