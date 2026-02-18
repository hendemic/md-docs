# mdocs overview
mdocs is a CLI app that converts markdown files into formatted pdf documents using typst. Currently supported on Linux and MacOS.

# Installation
To install run:
```
curl -fsSL https://raw.githubusercontent.com/hendemic/md-docs/main/dist/install.sh | sh
```

# Initialization and Templates
## Configuration
If you didn't initialize during install, `mdocs init` will set up the configuration file. 

Configuration is light and includes optional settings and the ability to specify custom repos.

*~/.config/mdocs/config.toml*:
```toml
# mdocs global configuration

# default_template = "resume-2-col"
# default_brand = "generic"
# author = "Your Name"

[[repos]]
name = "default"
url = "https://github.com/hendemic/md-docs-templates.git"
```

## Templates
mdocs allows use of templates and brands to structure markdown into formatted documents.

**Templates control layout, and brands control color and font**

Default templates are added from the templates repository upon app initialization. Current templates can be found [here](https://github.com/hendemic/md-docs-templates).

To update templates use `mdocs templates update` to pull the latest from the template repos. You can also update a single repo by following this command with the repos name in your configuration.

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
mdocs convert <file> [-t template] [-b brand] [-o output]    Convert a Markdown file to PDF
mdocs init                                                   Initialize config and install templates
mdocs new <template> [output_dir] [--name filename]          Create a new document from a starter file
mdocs config                                                 Show current configuration
mdocs update                                                 Update to latest version
mdocs update --check                                         Checks upstream for new version
mdocs uninstall                                              Uninstall app, default templates directory, and config file.

mdocs templates list                                         List available templates
mdocs templates install [name]                               Install templates (defaults to official repo)
mdocs templates update [name]                                Update installed templates
mdocs templates add <source>                                 Add a template source (git repo URL or local directory path) to config

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
 <br/>                    Add line break
```
