# test_compiler.py
# Unit tests for compiler.py

import shutil
from pathlib import Path

import pytest

from compiler import check_latex_engine, compile


class TestCheckLatexEngine:
    """Tests for check_latex_engine()."""

    def test_returns_version_when_found(self):
        # pdflatex should be installed on the test system
        result = check_latex_engine("pdflatex")
        if shutil.which("pdflatex"):
            assert result is not None
            assert isinstance(result, str)
        else:
            # Skip if pdflatex not installed
            pytest.skip("pdflatex not installed")

    def test_returns_none_when_not_found(self):
        result = check_latex_engine("nonexistent-latex-engine-xyz")
        assert result is None


class TestCompile:
    """Tests for compile()."""

    @pytest.fixture
    def minimal_latex(self):
        return r"""\documentclass{article}
\begin{document}
Hello, World!
\end{document}
"""

    @pytest.fixture
    def invalid_latex(self):
        return r"""\documentclass{article}
\begin{document}
\undefinedcommand
\end{document}
"""

    @pytest.fixture
    def minimal_template_dir(self, tmp_path):
        template_dir = tmp_path / "template"
        template_dir.mkdir()
        return template_dir

    @pytest.fixture
    def minimal_brand_dir(self, tmp_path):
        brand_dir = tmp_path / "brand"
        brand_dir.mkdir()
        (brand_dir / "brand.tex").write_text("% empty brand\n")
        return brand_dir

    @pytest.mark.skipif(
        not shutil.which("pdflatex"),
        reason="pdflatex not installed"
    )
    def test_compile_success(
        self, minimal_latex, minimal_template_dir, minimal_brand_dir, tmp_path
    ):
        output_path = tmp_path / "output.pdf"
        result = compile(
            minimal_latex,
            output_path,
            minimal_template_dir,
            minimal_brand_dir,
            engine="pdflatex",
        )
        assert result is True
        assert output_path.exists()
        assert output_path.stat().st_size > 0

    @pytest.mark.skipif(
        not shutil.which("pdflatex"),
        reason="pdflatex not installed"
    )
    def test_compile_failure_invalid_latex(
        self, invalid_latex, minimal_template_dir, minimal_brand_dir, tmp_path
    ):
        output_path = tmp_path / "output.pdf"
        result = compile(
            invalid_latex,
            output_path,
            minimal_template_dir,
            minimal_brand_dir,
            engine="pdflatex",
        )
        assert result is False

    @pytest.mark.skipif(
        not shutil.which("pdflatex"),
        reason="pdflatex not installed"
    )
    def test_compile_copies_brand_tex(
        self, minimal_latex, minimal_template_dir, tmp_path
    ):
        brand_dir = tmp_path / "brand"
        brand_dir.mkdir()
        (brand_dir / "brand.tex").write_text(r"\newcommand{\testcmd}{test}")

        output_path = tmp_path / "output.pdf"

        # This should work because brand.tex is copied
        latex_with_brand = r"""\documentclass{article}
\input{brand.tex}
\begin{document}
Hello!
\end{document}
"""
        result = compile(
            latex_with_brand,
            output_path,
            minimal_template_dir,
            brand_dir,
            engine="pdflatex",
        )
        assert result is True

    @pytest.mark.skipif(
        not shutil.which("pdflatex"),
        reason="pdflatex not installed"
    )
    def test_compile_copies_fonts_dir(
        self, minimal_latex, minimal_template_dir, tmp_path
    ):
        brand_dir = tmp_path / "brand"
        brand_dir.mkdir()
        (brand_dir / "brand.tex").write_text("% brand\n")

        # Create a fonts directory
        fonts_dir = brand_dir / "fonts"
        fonts_dir.mkdir()
        (fonts_dir / "test.txt").write_text("placeholder")

        output_path = tmp_path / "output.pdf"
        result = compile(
            minimal_latex,
            output_path,
            minimal_template_dir,
            brand_dir,
            engine="pdflatex",
        )
        # Should succeed even with fonts dir present
        assert result is True

    def test_compile_with_missing_engine(
        self, minimal_latex, minimal_template_dir, minimal_brand_dir, tmp_path
    ):
        output_path = tmp_path / "output.pdf"
        # Missing engine raises FileNotFoundError or returns False depending on implementation
        try:
            result = compile(
                minimal_latex,
                output_path,
                minimal_template_dir,
                minimal_brand_dir,
                engine="nonexistent-engine",
            )
            assert result is False
        except FileNotFoundError:
            # This is also acceptable behavior
            pass
