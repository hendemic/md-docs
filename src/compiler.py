# compiler.py
# Handles LaTeX to PDF compilation.
# Manages the temp directory, copies template support files and assets,
# runs the configured LaTeX engine, and places the final PDF at the output path.

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def check_latex_engine(engine: str = "pdflatex") -> str | None:
    """
    Verify a LaTeX engine is installed and reachable on PATH.

    Returns version string, or None if not found.
    """
    try:
        result = subprocess.run(
            [engine, "--version"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return result.stdout.split("\n", 1)[0]
        return None
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return None


def compile(
    latex_source: str,
    output_path: Path,
    template_dir: Path,
    brand_dir: Path,
    engine: str = "pdflatex",
) -> bool:
    """
    Compile LaTeX source to PDF.

    Args:
        latex_source: Complete rendered LaTeX document.
        output_path: Where to write the final PDF.
        template_dir: Resolved template directory (for assets).
        brand_dir: Resolved brand directory (for brand.tex).
        engine: LaTeX engine to use (pdflatex, xelatex, lualatex).

    Returns:
        True on success, False on failure.
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        tex_file = tmpdir_path / "document.tex"

        # Copy brand.tex into the temp dir
        brand_tex = brand_dir / "brand.tex"
        if brand_tex.is_file():
            shutil.copy(brand_tex, tmpdir_path / "brand.tex")

        # Copy template assets (images, supplementary .tex files)
        assets_dir = template_dir / "assets"
        if assets_dir.is_dir():
            for asset in assets_dir.iterdir():
                if asset.is_file():
                    shutil.copy(asset, tmpdir_path / asset.name)

        # Copy brand fonts directory if present
        brand_fonts = brand_dir / "fonts"
        if brand_fonts.is_dir():
            shutil.copytree(brand_fonts, tmpdir_path / "fonts")

        # Write the rendered LaTeX source
        tex_file.write_text(latex_source, encoding="utf-8")

        # Run engine twice for references/TOC
        for run in range(2):
            result = subprocess.run(
                [
                    engine,
                    "-interaction=nonstopmode",
                    "-halt-on-error",
                    "document.tex",
                ],
                cwd=tmpdir,
                capture_output=True,
                text=True,
            )

            if result.returncode != 0:
                print(f"LaTeX compilation failed (pass {run + 1}):", file=sys.stderr)
                print(result.stdout, file=sys.stderr)

                # Save .tex next to intended output for debugging
                debug_path = output_path.with_suffix(".tex")
                shutil.copy(tex_file, debug_path)
                print(f"Debug LaTeX source saved to: {debug_path}", file=sys.stderr)
                return False

        # Copy PDF to output location
        pdf_file = tmpdir_path / "document.pdf"
        if pdf_file.exists():
            output_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(pdf_file, output_path)
            return True

        print("PDF was not generated.", file=sys.stderr)
        return False
