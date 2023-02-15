# MC-Repack
An experimental repacking tool for Minecraft mods to optimize size and loading speed of mods.

## Features
- Minifying JSON files, using `serde-json`
- Optimizing PNG files, using `oxipng`
- Optimizing TOML files, using `toml`

There are other, less important functions like stripping Unicode BOM and removing comment lines in `.cfg` files.

## Usage
Currently the only way to get this app is to use Cargo:
```sh
cargo install --git https://github.com/szeweq/mc-repack
```

After installation, the tool can be used by typing the following command:
```sh
mc-repack <file|directory>
```
When a file path is provided, then MC-Repack will repack the file contents into a `$repack.jar`. If a path is a directory, then all files inside (non-recursive) will be repacked.

## Why?
MC-Repack is meant to show how many Minecraft mods come with unoptimized files (and I don't mean just pretty-printed JSON files). You will be surprised that in some cases a PNG file's metadata added by Photoshop is much larger than its content.

One other inportant thing is that MC-Repack determines if a file really needs to be compressed. **Most PNG files and smaller JSON files will usually be stored uncompressed.** This kind of operation saves bytes if a "compressed" form is larger than original. Also, uncompressed data can be loaded faster.

This is a great tool that can be helpful for:
- Mod developers – they can provide mods with smaller file sizes, optimized PNGs and correctly formatted JSONs
- Players and server owners – optimized and repacked files can speed up Minecraft load time while using less memory.

MC-Repack shows all errors happened during repacking. Most of them are errors that can be simply ignored (like `trailing comma at line X column Y`).

## How to contribute?
The easiest way to contribute is to share this to others.

## Can I use it outside Minecraft?
Yes. The tool does not recognize that a `.jar` file is not a Minecraft mod.

## Future plans
- Aggresive mode – minify JavaScript and shader files (potentially breaking debugging).
- Remove unwanted files – some project files (from Blender, Photoshop, etc.) are mistakenly packed in mods. This operation will detect and remove (ignore while repacking) these files.