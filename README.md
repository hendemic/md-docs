# mdocs overview
mdocs is a CLI app that converts markdown files into formatted pdf documents using typst.

# Installation
This app is still in active development. Until install, updates, and uninstall are developed, pull this repo, use `cargo build --release`, and move the binary to your bin to use the CLI. 

Future versions will include install and updates via the app and published binaries for Linux and MacOS.

# Initialization and Templates
## Configuration
`mdocs init` will set up the configuration file. 

Configuration is light and includes optional settings. Future versions will support custom template repos and output directories. 

*~/.config/md-docs/config.toml*:
```toml
# md-docs global configuration

# default_template = "resume-2-col"
# default_brand = "generic"
# author = "Your Name"
```

## Templates
mdocs allows use of templates and brands to structure markdown into formatted documents.

**Templates control layout, and brands control color and font**

Default templates are added from the templates repository upon app initialization (`mdocs init`). Current templates can be found here: https://github.com/hendemic/md-docs-templates

To update templates use `mdocs templates update` to pull the latest from the template repo. In the future, custom repos will be supported and the user will be able to specify the repo to pull from or pull from all.

Brands are specified and prioritized in the following way
1. CLI flag -b takes top priority
2. default_brand in your config if you've specified one (for example, if you want the default to be your company's brand)
3. the templates default_brand defined in each templates metadata.toml
4. If none of these are defined, the CLI will prompt for a selection


# Usage
## Basic usage
To convert a doc, simply type `mdocs <filepath>`. You'll be prompted for a template if you don't pass one in as an arg.

## Commands

```
mdocs convert <file> [-t template] [-b brand] [-o output]   Convert a Markdown file to PDF
mdocs init                                                   Initialize config and install templates
mdocs new <template> [output_dir] [--name filename]          Create a new document from a starter file
mdocs config                                                 Show current configuration

mdocs templates list                                         List available templates
mdocs templates install [url]                                Install templates (defaults to official repo)
mdocs templates update [name]                                Update installed templates
mdocs templates remove <name>                                Remove a template

mdocs brands list                                            List available brands
```

Options:
- `-v, --verbose` — Enable verbose output
- `-h, --help` — Print help
- `-V, --version` — Print version

## Modifiers
Modifiers are special markers you can place in your markdown to control layout. Not all templates support every modifier — unsupported ones are either removed or replaced depending on the template. See `modifiers.toml` for the full list.

```
<!-- COLUMNS_START -->    Split content into header (above) and columns (below)
<!-- COLUMN_BREAK -->     Switch to the next column
<!-- PAGEBREAK -->        Force a new page
<!-- BOTTOM -->           Push remaining content to the bottom of the page
 /|                       Inline left/right alignment (e.g., title /| date)
```
