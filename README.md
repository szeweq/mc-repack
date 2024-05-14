# MC-Repack
![Logo](https://raw.githubusercontent.com/szeweq/mc-repack/master/mc-repack-logo.png)

[![Rust Build](https://github.com/szeweq/mc-repack/actions/workflows/rust-build.yml/badge.svg)](https://github.com/szeweq/mc-repack/actions/workflows/rust-build.yml)
[![dependency status](https://deps.rs/repo/github/szeweq/mc-repack/status.svg)](https://deps.rs/repo/github/szeweq/mc-repack)

A repacking tool for Minecraft mods and resource packs to optimize size and loading speed.

## Features
- Minifying JSON files, using `serde-json`, and removing comments
- Optimizing PNG files, using `oxipng`
- Optimizing TOML files, using `toml`
- Optimizing NBT files
- Removing unwanted files – some project files (from Blender, Photoshop, etc.) are mistakenly packed in mods. This operation will detect and remove (ignore while repacking) these files.
- Stripping Unicode BOM
- Removing comment lines in many file types: `.cfg, .obj, .mtl, .zs, .vsh, .fsh`
- Recompressing files more efficiently
- Now with Zopfli support (slower, but better compression)

## Comparison table
These mods are tested and repacked by MC-Repack with the following results:
| File name | Original | Optimized |
|----|----:|----:|
| minecolonies-1.19.2-1.0.1247-BETA.jar | 72.8 MB | 63.7 MB |
| twilightforest-1.19.3-4.2.1549-universal.jar | 22.5 MB | 21.9 MB |
| TConstruct-1.18.2-3.6.3.111.jar | 15.2 MB | 14.0 MB |
| BloodMagic-1.18.2-3.2.6-41.jar | 13.6 MB | 11.9 MB |
| create-1.19.2-0.5.0.i.jar | 13.1 MB | 12.8 MB |
| Botania-1.19.2-437-FORGE.jar | 10.9 MB | 10.1 MB |
| ImmersiveEngineering-1.19.3-9.3.0-163.jar | 10.3 MB | 10.0 MB |
| thermal_foundation-1.19.2-10.2.0.47.jar | 4.58 MB | 4.38 MB |
| cfm-7.0.0-pre35-1.19.3.jar | 2.11 MB | 1.92 MB |

More mods are available on [My Website](https://szeweq.xyz/mc-repack/mods).

## Installation
The fastest way is to get the latest version from [Releases](https://github.com/szeweq/mc-repack/releases/latest) page.

You can also install this app by using Cargo:
```sh
cargo install mc-repack
```

If you want to test the latest commit directly from this repo:
```sh
cargo install --git https://github.com/szeweq/mc-repack
```

## Usage
After installation, the tool can be used by typing the following command:
```sh
mc-repack <file|directory>
```
When a file path is provided, then MC-Repack will repack the file contents (adds `$repack` on new archive). If a path is a directory, then all files inside (non-recursive) will be repacked.

More options are provided by typing `mc-repack --help` in a shell/terminal.

## Library
MC-Repack can be also purposed for creating your own version of a repacking tool. See [`mc-repack-core` crate on crates.io](https://crates.io/crates/mc-repack-core) for more technical details.

## Why?
MC-Repack is meant to show how many Minecraft mods and resource packs come with unoptimized files (and I don't mean just pretty-printed JSON files). You will be surprised that in some cases a PNG file's metadata added by Photoshop is much larger than its content.

One other inportant thing is that MC-Repack determines if a file really needs to be compressed. **Most PNG files and smaller JSON files will usually be stored uncompressed.** This kind of operation saves bytes if a "compressed" form is larger than original. Also, uncompressed data can be loaded faster.

This is a great tool that can be helpful for:
- Mod developers and resource pack makers – they can provide mods with smaller file sizes, optimized PNGs and correctly formatted JSONs
- Players and server owners – optimized and repacked files can speed up Minecraft load time while using less memory.

MC-Repack shows all errors happened during repacking. Most of them are errors that can be simply ignored (like `trailing comma at line X column Y`).

## How to contribute?
The easiest way to contribute is to share this with others on social media.

There is a lot of things that should be fixed or optimized. New feature ideas are welcome, just file an issue.

## Can I use it outside Minecraft?
Sure! The tool does not recognize that an archive is not a Minecraft mod nor a resource pack (yet).
