# converter.py
# Custom mistune renderer that converts Markdown AST to LaTeX.
# Each Markdown element (headings, lists, tables, code blocks, etc.)
# maps to a method that returns the equivalent LaTeX string.
# Also provides escape_latex() for sanitizing special characters.
#
# Modifier handling:
#   Block modifiers (HTML comments like <!-- COLUMN_BREAK -->) are resolved
#   in block_html() using a modifier map passed at init.
#   Inline modifiers (like /|) are resolved in postprocess_latex() after
#   rendering, since they appear as plain text inside parsed inline content.

import re

from mistune import BaseRenderer


def postprocess_latex(latex: str, modifiers: dict) -> str:
    """
    Post-process rendered LaTeX for inline modifiers.

    Applies resolved inline modifier substitutions (e.g., /| -> \\hfill
    or /| -> newline if ignored by the template).
    """
    for mod in modifiers.values():
        if mod["type"] != "inline":
            continue
        marker = mod["marker"]
        latex_out = mod["latex"]
        if latex_out is None:
            # on_ignore = "keep" — leave as-is
            continue
        # Escape the marker for use in regex, allow flexible whitespace
        pattern = r"\s*" + re.escape(marker.strip()) + r"\s*"
        replacement = latex_out if latex_out else ""
        # re.sub interprets backslashes in replacement, so escape them
        latex = re.sub(pattern, replacement.replace("\\", "\\\\"), latex)

    # After inline modifier substitution, ensure the date/title line gets a
    # hard break before the description text that follows on the next line.
    # Matches lines ending with \emph{...} (date) or containing \hfill,
    # followed by a non-blank, non-command line.
    latex = re.sub(
        r"(\\hfill\s+[^\n]+)\n(?!\n)(?!\\)",
        r"\1\\\\\n",
        latex,
    )
    latex = re.sub(
        r"(\\emph\{[^}]+\})\n(?!\n)(?!\\)",
        r"\1\\\\\n",
        latex,
    )
    # Also break after \textbf{...} when followed by \emph{...} on the next line
    # (e.g., job title / degree on one line, date on the next)
    latex = re.sub(
        r"(\\textbf\{[^}]+\})\n(\\emph)",
        r"\1\\\\\n\2",
        latex,
    )

    return latex


def escape_latex(text: str) -> str:
    """
    Escape special LaTeX characters in plain text.
    Order matters — backslash must be escaped first.
    """
    replacements = [
        ("\\", r"\textbackslash{}"),
        ("&", r"\&"),
        ("%", r"\%"),
        ("$", r"\$"),
        ("#", r"\#"),
        ("_", r"\_"),
        ("{", r"\{"),
        ("}", r"\}"),
        ("~", r"\textasciitilde{}"),
        ("^", r"\textasciicircum{}"),
    ]
    for old, new in replacements:
        text = text.replace(old, new)
    return text


class LatexRenderer(BaseRenderer):
    """Custom mistune renderer that outputs LaTeX."""

    NAME = "latex"

    def __init__(self, modifiers: dict | None = None):
        super().__init__()
        # Build a lookup from HTML comment marker -> latex output for block modifiers
        self._block_modifiers = {}
        if modifiers:
            for mod in modifiers.values():
                if mod["type"] == "block":
                    self._block_modifiers[mod["marker"]] = mod["latex"]

    def render_children(self, token, state):
        children = token.get("children")
        if children:
            return self.render_tokens(children, state)
        return ""

    # -- Block-level elements --

    def paragraph(self, token, state) -> str:
        text = self.render_children(token, state)
        return f"{text}\n\n"

    def heading(self, token, state) -> str:
        level = token["attrs"]["level"]
        text = self.render_children(token, state)
        commands = {
            1: "section",
            2: "subsection",
            3: "subsubsection",
            4: "paragraph",
            5: "subparagraph",
            6: "subparagraph",
        }
        cmd = commands.get(level, "paragraph")
        return f"\\{cmd}{{{text}}}\n\n"

    def blank_line(self, token, state) -> str:
        return ""

    def thematic_break(self, token, state) -> str:
        return "\\bigskip\\hrule\\bigskip\n\n"

    def block_code(self, token, state) -> str:
        code = token.get("raw", "")
        info = token.get("attrs", {}).get("info", "")
        if info:
            return f"\\begin{{lstlisting}}[language={info}]\n{code}\\end{{lstlisting}}\n\n"
        return f"\\begin{{lstlisting}}\n{code}\\end{{lstlisting}}\n\n"

    def block_quote(self, token, state) -> str:
        text = self.render_children(token, state)
        return f"\\begin{{quote}}\n{text}\\end{{quote}}\n\n"

    def list(self, token, state) -> str:
        ordered = token["attrs"].get("ordered", False)
        env = "enumerate" if ordered else "itemize"
        text = self.render_children(token, state)
        return f"\\begin{{{env}}}\n{text}\\end{{{env}}}\n\n"

    def list_item(self, token, state) -> str:
        text = self.render_children(token, state).rstrip()
        return f"\\item {text}\n"

    def block_text(self, token, state) -> str:
        return self.render_children(token, state)

    def block_html(self, token, state) -> str:
        raw = token.get("raw", "").strip()
        if raw in self._block_modifiers:
            latex = self._block_modifiers[raw]
            if latex is None:
                # on_ignore = "keep" — leave raw text
                return raw + "\n\n"
            if latex:
                return latex + "\n\n"
            return ""
        return ""

    def block_error(self, token, state) -> str:
        return ""

    def inline_html(self, token, state) -> str:
        raw = token.get("raw", "").strip().lower()
        if raw == "<br>" or raw == "<br/>" or raw == "<br />":
            return "\\vspace{\\baselineskip}\n"
        return ""

    # -- Inline elements --

    def text(self, token, state) -> str:
        return escape_latex(token.get("raw", ""))

    def emphasis(self, token, state) -> str:
        text = self.render_children(token, state)
        return f"\\emph{{{text}}}"

    def strong(self, token, state) -> str:
        text = self.render_children(token, state)
        return f"\\textbf{{{text}}}"

    def codespan(self, token, state) -> str:
        raw = token.get("raw", "")
        escaped = raw.replace("\\", r"\textbackslash{}")
        escaped = escaped.replace("{", r"\{")
        escaped = escaped.replace("}", r"\}")
        return f"\\texttt{{{escaped}}}"

    def link(self, token, state) -> str:
        text = self.render_children(token, state)
        url = token.get("attrs", {}).get("url", "")
        escaped_url = url.replace("%", r"\%").replace("#", r"\#")
        return f"\\href{{{escaped_url}}}{{{text}}}"

    def image(self, token, state) -> str:
        alt = token.get("attrs", {}).get("alt", "")
        url = token.get("attrs", {}).get("url", "")
        return (
            f"\\begin{{figure}}[h]\n"
            f"\\centering\n"
            f"\\includegraphics[width=0.8\\textwidth]{{{url}}}\n"
            f"\\caption{{{escape_latex(alt)}}}\n"
            f"\\end{{figure}}\n"
        )

    def linebreak(self, token, state) -> str:
        return "\\\\\n"

    def softbreak(self, token, state) -> str:
        return "\n"

    def strikethrough(self, token, state) -> str:
        text = self.render_children(token, state)
        return f"\\sout{{{text}}}"

    # -- Table elements --

    def table(self, token, state) -> str:
        children = token.get("children", [])
        num_cols = 1
        if children and children[0].get("type") == "table_head":
            num_cols = len(children[0].get("children", []))

        col_spec = "|" + "X|" * num_cols
        content = self.render_children(token, state)
        return (
            f"\\begin{{center}}\n\\small\n"
            f"\\begin{{tabularx}}{{\\textwidth}}{{{col_spec}}}\n"
            f"\\hline\n{content}\\hline\n"
            f"\\end{{tabularx}}\n\\end{{center}}\n\n"
        )

    def table_head(self, token, state) -> str:
        children = token.get("children", [])
        cells = []
        for child in children:
            cell_text = self.render_children(child, state)
            cells.append(f"\\textbf{{{cell_text}}}")
        return " & ".join(cells) + " \\\\\n\\hline\n"

    def table_body(self, token, state) -> str:
        return self.render_children(token, state)

    def table_row(self, token, state) -> str:
        children = token.get("children", [])
        cells = [self.render_children(child, state) for child in children]
        return " & ".join(cells) + " \\\\\n"

    def table_cell(self, token, state) -> str:
        return self.render_children(token, state)
