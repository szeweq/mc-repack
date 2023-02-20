# MC-Repack
A repacking tool for Minecraft mods to optimize size and loading speed of mods.

## Features
- Minifying JSON files, using `serde-json`
- Optimizing PNG files, using `oxipng`
- Optimizing TOML files, using `toml`
- Removing unwanted files – some project files (from Blender, Photoshop, etc.) are mistakenly packed in mods. This operation will detect and remove (ignore while repacking) these files.
- Stripping Unicode BOM
- Removing comment lines in in `.cfg` files

## Comparison table
These mods were tested and repacked by MC-Repack with the following results:
| File name | Before | After |
|----|----:|----:|
| minecolonies-1.19.2-1.0.1247-BETA.jar | 72.8 MB | 65.2 MB |
| twilightforest-1.19.3-4.2.1549-universal.jar | 22.5 MB | 22.0 MB |
| create-1.19.2-0.5.0.i.jar | 13.1 MB | 12.8 MB |

## Installation
Currently the only way to get this app is to use Cargo:
```sh
cargo install mc-repack
```

If you want to test the latest version directly from this repo:
```sh
cargo install --git https://github.com/szeweq/mc-repack
```

## Usage
After installation, the tool can be used by typing the following command:
```sh
mc-repack <file|directory>
```
When a file path is provided, then MC-Repack will repack the file contents into a `$repack.jar`. If a path is a directory, then all files inside (non-recursive) will be repacked.

More options are provided by typing `mc-repack --help` in a shell/terminal.

## Why?
MC-Repack is meant to show how many Minecraft mods come with unoptimized files (and I don't mean just pretty-printed JSON files). You will be surprised that in some cases a PNG file's metadata added by Photoshop is much larger than its content.

One other inportant thing is that MC-Repack determines if a file really needs to be compressed. **Most PNG files and smaller JSON files will usually be stored uncompressed.** This kind of operation saves bytes if a "compressed" form is larger than original. Also, uncompressed data can be loaded faster.

This is a great tool that can be helpful for:
- Mod developers – they can provide mods with smaller file sizes, optimized PNGs and correctly formatted JSONs
- Players and server owners – optimized and repacked files can speed up Minecraft load time while using less memory.

MC-Repack shows all errors happened during repacking. Most of them are errors that can be simply ignored (like `trailing comma at line X column Y`).

## How to contribute?
The easiest way to contribute is to share this with others on social media.

There is a lot of things that should be fixed or optimized. New feature ideas are welcome, just file an issue.

## Can I use it outside Minecraft?
Yes. The tool does not recognize that a `.jar` file is not a Minecraft mod.

## Future plans
- Aggresive mode – minify JavaScript and shader files (potentially breaking debugging).
- Strip unwanted data in JSON files
- Recompress `.class` files more efficiently