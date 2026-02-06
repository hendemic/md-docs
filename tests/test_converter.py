# test_converter.py
# Unit tests for converter.py

import pytest
import mistune

from converter import escape_latex, LatexRenderer, postprocess_latex


class TestEscapeLaTeX:
    """Tests for escape_latex()."""

    def test_escapes_ampersand(self):
        assert escape_latex("A & B") == r"A \& B"

    def test_escapes_percent(self):
        assert escape_latex("100%") == r"100\%"

    def test_escapes_dollar(self):
        assert escape_latex("$100") == r"\$100"

    def test_escapes_hash(self):
        assert escape_latex("#1") == r"\#1"

    def test_escapes_underscore(self):
        assert escape_latex("foo_bar") == r"foo\_bar"

    def test_escapes_braces(self):
        assert escape_latex("{a}") == r"\{a\}"

    def test_escapes_tilde(self):
        assert escape_latex("~") == r"\textasciitilde{}"

    def test_escapes_caret(self):
        assert escape_latex("^") == r"\textasciicircum{}"

    def test_escapes_backslash(self):
        # Backslash gets escaped, then braces in the result also get escaped
        result = escape_latex("\\")
        assert "textbackslash" in result

    def test_escapes_multiple_special_chars(self):
        result = escape_latex("Cost: $100 & 50% off #1")
        assert r"\$" in result
        assert r"\&" in result
        assert r"\%" in result
        assert r"\#" in result

    def test_empty_string(self):
        assert escape_latex("") == ""

    def test_no_special_chars(self):
        assert escape_latex("Hello World") == "Hello World"


class TestLatexRendererIntegration:
    """Integration tests for LatexRenderer using full markdown conversion."""

    @pytest.fixture
    def convert(self):
        """Return a function that converts markdown to LaTeX."""
        def _convert(markdown_text, modifiers=None):
            modifiers = modifiers or {}
            renderer = LatexRenderer(modifiers=modifiers)
            md = mistune.create_markdown(
                renderer=renderer,
                plugins=["table", "strikethrough"],
            )
            return md(markdown_text)
        return _convert

    def test_paragraph(self, convert):
        result = convert("Hello world")
        assert "Hello world" in result

    def test_heading_h1(self, convert):
        result = convert("# Title")
        assert r"\section{Title}" in result

    def test_heading_h2(self, convert):
        result = convert("## Subtitle")
        assert r"\subsection{Subtitle}" in result

    def test_heading_h3(self, convert):
        result = convert("### Section")
        assert r"\subsubsection{Section}" in result

    def test_heading_h4(self, convert):
        result = convert("#### Subsection")
        assert r"\paragraph{Subsection}" in result

    def test_strong(self, convert):
        result = convert("**bold text**")
        assert r"\textbf{bold text}" in result

    def test_emphasis(self, convert):
        result = convert("*italic text*")
        assert r"\emph{italic text}" in result

    def test_link(self, convert):
        result = convert("[Example](https://example.com)")
        assert r"\href{https://example.com}{Example}" in result

    def test_list_unordered(self, convert):
        result = convert("- Item one\n- Item two")
        assert r"\begin{itemize}" in result
        assert r"\end{itemize}" in result
        assert r"\item" in result

    def test_list_ordered(self, convert):
        result = convert("1. First\n2. Second")
        assert r"\begin{enumerate}" in result
        assert r"\end{enumerate}" in result

    def test_codespan(self, convert):
        result = convert("`code`")
        assert r"\texttt{" in result

    def test_block_html_modifier(self, convert):
        modifiers = {
            "column_break": {
                "marker": "<!-- COLUMN_BREAK -->",
                "latex": r"\switchcolumn",
                "type": "block",
            },
        }
        result = convert("<!-- COLUMN_BREAK -->", modifiers)
        assert r"\switchcolumn" in result

    def test_block_html_unknown_removed(self, convert):
        result = convert("<!-- unknown comment -->")
        # Unknown HTML comments should be removed or empty
        assert "unknown comment" not in result

    def test_mixed_content(self, convert):
        md = """# Title

This is a **bold** paragraph with *italic* text.

- Item 1
- Item 2
"""
        result = convert(md)
        assert r"\section{Title}" in result
        assert r"\textbf{bold}" in result
        assert r"\emph{italic}" in result
        assert r"\begin{itemize}" in result


class TestPostprocessLatex:
    """Tests for postprocess_latex()."""

    @pytest.fixture
    def modifiers_with_hfill(self):
        return {
            "date_separator": {
                "marker": " /| ",
                "latex": r" \hfill ",
                "type": "inline",
            },
        }

    def test_replaces_inline_modifier(self, modifiers_with_hfill):
        text = "Title /| Date"
        result = postprocess_latex(text, modifiers_with_hfill)
        assert r"\hfill" in result
        assert "/|" not in result

    def test_hfill_adds_linebreak(self, modifiers_with_hfill):
        text = "Title /| Date\n\nNext paragraph"
        result = postprocess_latex(text, modifiers_with_hfill)
        # After hfill line, there should be \\ before next content
        assert r"\hfill" in result

    def test_empty_modifiers(self):
        text = "Plain text"
        result = postprocess_latex(text, {})
        assert result == text

    def test_preserves_content(self, modifiers_with_hfill):
        text = "Some text without modifiers"
        result = postprocess_latex(text, modifiers_with_hfill)
        assert "Some text without modifiers" in result
