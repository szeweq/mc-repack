# The `mc-repack-core` create

This crate is a part of MC-Repack project, available on GitHub ([see here](https://github.com/szeweq/mc-repack)).

## Features
- Minifying JSON files, using `serde-json`, and removing comments
- Optimizing PNG files, using `oxipng`
- Optimizing TOML files, using `toml`
- Removing unwanted files – some project files (from Blender, Photoshop, etc.) are mistakenly packed in mods. This operation will detect and remove (ignore while repacking) these files.
- Stripping Unicode BOM
- Removing comment lines in many file types: `.cfg, .obj, .mtl, .zs, .vsh, .fsh`
- Recompressing files more efficiently

For more info, visit the [MC-Repack webpage](https://szeweq.xyz/mc-repack)