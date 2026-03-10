# pyrite

A small wrapper around Gentoo’s Portage tools written in Rust.

`pyrite` provides a **pacman / paru-style interface** for Gentoo by wrapping `emerge`, `eix`, and `eselect`. The goal is simple: let you manage a Gentoo system using commands that feel familiar if you come from Arch.

## Why I Made This

I spent years on Arch using `pacman` and later `paru`. When I switched to daily-driving Gentoo, the tooling itself wasn’t the problem. Portage is solid. The friction was mostly muscle memory.

Instead of retraining myself every time I installed, searched, or upgraded packages, I wrote `pyrite`. It translates a pacman-style workflow into the equivalent Portage commands under the hood.

Nothing fancy. Just a thin wrapper so Gentoo feels a little more like home if you’re coming from Arch.

## Installation

### Dependencies

* `app-portage/eix`
* `app-admin/eselect`
* `dev-lang/rust(-bin)`

### Build

```bash
git clone https://github.com/Gur0v/pyrite.git
cd pyrite
make
sudo make install
```

The Makefile installs:

* `pyrite` → `/usr/local/bin/pyrite`
* a **symlink to `/usr/local/bin/paru`**

This lets you keep using the `paru` command out of habit, which makes the transition from Arch to Gentoo a lot smoother.

## Examples

```bash
pyrite                 # Check for unread news
pyrite -Syu            # Sync and upgrade everything
pyrite -S firefox      # Install firefox
pyrite -Ss browser     # Search for packages with "browser" in the name
pyrite -R firefox      # Remove firefox
pyrite -Rdd firefox    # Force remove (ignore dependencies)
```

## Help

To see the full list of supported operations and flags:

```bash
pyrite --help
```

## License

[GPLv3](LICENSE)
