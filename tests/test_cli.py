# test_cli.py
# Integration tests for the CLI.

import shutil
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch

import pytest

from cli import (
    build_parser,
    parse_frontmatter,
    cmd_doctor,
    cmd_templates_list,
    cmd_brands_list,
    get_version,
)


class TestParseArgs:
    """Tests for argument parsing."""

    def test_no_args_sets_command_none(self):
        parser = build_parser()
        args = parser.parse_args([])
        assert args.command is None

    def test_convert_command(self):
        parser = build_parser()
        args = parser.parse_args(["convert", "input.md"])
        assert args.command == "convert"
        assert args.input == "input.md"

    def test_convert_with_template(self):
        parser = build_parser()
        args = parser.parse_args(["convert", "input.md", "-t", "resume"])
        assert args.template == "resume"

    def test_convert_with_brand(self):
        parser = build_parser()
        args = parser.parse_args(["convert", "input.md", "-b", "generic"])
        assert args.brand == "generic"

    def test_convert_with_output(self):
        parser = build_parser()
        args = parser.parse_args(["convert", "input.md", "-o", "output.pdf"])
        assert args.output == "output.pdf"

    def test_templates_list(self):
        parser = build_parser()
        args = parser.parse_args(["templates", "list"])
        assert args.command == "templates"
        assert args.templates_cmd == "list"

    def test_templates_install(self):
        parser = build_parser()
        args = parser.parse_args(["templates", "install"])
        assert args.command == "templates"
        assert args.templates_cmd == "install"

    def test_templates_remove(self):
        parser = build_parser()
        args = parser.parse_args(["templates", "remove", "test-template"])
        assert args.command == "templates"
        assert args.templates_cmd == "remove"
        assert args.name == "test-template"

    def test_brands_list(self):
        parser = build_parser()
        args = parser.parse_args(["brands", "list"])
        assert args.command == "brands"
        assert args.brands_cmd == "list"

    def test_doctor(self):
        parser = build_parser()
        args = parser.parse_args(["doctor"])
        assert args.command == "doctor"


class TestParseFrontmatter:
    """Tests for parse_frontmatter()."""

    def test_extracts_yaml_frontmatter(self):
        content = """---
title: Test
author: Me
---

Body content here.
"""
        metadata, body = parse_frontmatter(content)
        assert metadata["title"] == "Test"
        assert metadata["author"] == "Me"
        assert "Body content here." in body

    def test_no_frontmatter(self):
        content = "Just some content without frontmatter."
        metadata, body = parse_frontmatter(content)
        assert metadata == {}
        assert body == content

    def test_empty_frontmatter(self):
        content = """---
---

Body here.
"""
        metadata, body = parse_frontmatter(content)
        assert metadata == {}
        assert "Body here." in body

    def test_invalid_yaml_returns_empty(self):
        content = """---
invalid: yaml: syntax: here
---

Body.
"""
        metadata, body = parse_frontmatter(content)
        # Should return empty dict on parse error
        assert metadata == {}


class TestGetVersion:
    """Tests for get_version()."""

    def test_returns_dev_when_no_version_file(self, tmp_path):
        with patch("cli.DATA_DIR", tmp_path):
            result = get_version()
            assert result == "dev"

    def test_returns_version_from_file(self, tmp_path):
        version_file = tmp_path / ".version"
        version_file.write_text("1.2.3\n")
        with patch("cli.DATA_DIR", tmp_path):
            result = get_version()
            assert result == "1.2.3"


class TestCmdDoctor:
    """Tests for cmd_doctor()."""

    def test_doctor_returns_zero_when_latex_found(self, mock_config, capsys):
        if not shutil.which("pdflatex"):
            pytest.skip("pdflatex not installed")

        # Create a mock args object
        class Args:
            pass

        result = cmd_doctor(Args(), mock_config)
        captured = capsys.readouterr()

        assert "pdflatex" in captured.out
        # Should return 0 if pdflatex found
        assert result == 0

    def test_doctor_returns_one_when_latex_missing(self, mock_config, capsys):
        class Args:
            pass

        config = {**mock_config, "latex_engine": "nonexistent-engine"}
        result = cmd_doctor(Args(), config)
        captured = capsys.readouterr()

        assert "NOT FOUND" in captured.out
        assert result == 1


class TestCmdTemplatesList:
    """Tests for cmd_templates_list()."""

    def test_empty_templates(self, mock_config, capsys):
        class Args:
            pass

        result = cmd_templates_list(Args(), mock_config)
        captured = capsys.readouterr()

        assert "No templates installed" in captured.out
        assert result == 0

    def test_lists_templates(self, mock_config, sample_template, capsys):
        class Args:
            pass

        result = cmd_templates_list(Args(), mock_config)
        captured = capsys.readouterr()

        assert "test-template" in captured.out
        assert "Test Template" in captured.out
        assert result == 0


class TestCmdBrandsList:
    """Tests for cmd_brands_list()."""

    def test_empty_brands(self, mock_config, capsys):
        class Args:
            pass

        result = cmd_brands_list(Args(), mock_config)
        captured = capsys.readouterr()

        assert "No brands installed" in captured.out
        assert result == 0

    def test_lists_brands(self, mock_config, sample_brand, capsys):
        class Args:
            pass

        result = cmd_brands_list(Args(), mock_config)
        captured = capsys.readouterr()

        assert "test-brand" in captured.out
        assert "Test Brand" in captured.out
        assert result == 0


class TestCLIIntegration:
    """Integration tests running the actual CLI."""

    @pytest.fixture
    def cli_path(self):
        return Path(__file__).parent.parent / "src" / "cli.py"

    def test_cli_help(self, cli_path):
        result = subprocess.run(
            [sys.executable, str(cli_path), "--help"],
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0
        assert "mdocs" in result.stdout
        assert "convert" in result.stdout

    def test_cli_version(self, cli_path):
        result = subprocess.run(
            [sys.executable, str(cli_path), "--version"],
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0
        assert "mdocs" in result.stdout

    def test_cli_convert_missing_file(self, cli_path, tmp_path):
        result = subprocess.run(
            [sys.executable, str(cli_path), "convert", "nonexistent.md"],
            capture_output=True,
            text=True,
            cwd=tmp_path,
        )
        assert result.returncode == 1
        assert "not found" in result.stderr.lower() or "error" in result.stderr.lower()

    def test_cli_doctor(self, cli_path):
        result = subprocess.run(
            [sys.executable, str(cli_path), "doctor"],
            capture_output=True,
            text=True,
        )
        # Should run without crashing
        assert result.returncode in [0, 1]  # 0 if pdflatex found, 1 if not
        assert "pdflatex" in result.stdout or "Templates" in result.stdout
