
/// An enum containing all "known" file formats that can be processed by this library
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KnownFmt {
    /// JavaScript Object Notation, also known as JSON
    Json,
    /// Tom's Obvious Minimal Language, also known as TOML
    Toml,
    /// Portable Network Graphics, also known as PNG
    Png,
    /// Ogg Vorbis (it may not be an audio file)
    Ogg,
    /// Named Binary Tag, also known as NBT
    Nbt,
    /// Configuration file
    Cfg,
    /// Wavefront .obj file
    Obj,
    /// Wavefront .mtl file
    Mtl,
    /// GLSL fragment shader
    Fsh,
    /// GLSL vertex shader
    Vsh,
    /// JavaScript
    Js,
    /// ZenScript (ZS), format used by CraftTweaker
    Zs,
    /// JAR archive
    Jar,
    /// Java Manifest file
    Mf,
    /// Any type format with maximum length of 3 (unused bytes are marked as zeroes)
    Other([u8; 3])
}
impl KnownFmt {
    /// Return a `KnownFmt` based on file extension.
    #[must_use]
    pub fn by_extension(ftype: &str) -> Option<Self> {
        Some(match ftype.to_ascii_lowercase().as_str() {
            "json" | "mcmeta" => Self::Json,
            "toml" => Self::Toml,
            "png" => Self::Png,
            "ogg" => Self::Ogg,
            "nbt" | "blueprint" => Self::Nbt,
            "cfg" => Self::Cfg,
            "obj" => Self::Obj,
            "mtl" => Self::Mtl,
            "fsh" => Self::Fsh,
            "vsh" => Self::Vsh,
            "js" => Self::Js,
            "zs" => Self::Zs,
            "jar" => Self::Jar,
            "mf" => Self::Mf,
            x => match x.as_bytes() {
                [a] => Self::Other([*a, 0, 0]),
                [a, b] => Self::Other([*a, *b, 0]),
                [a, b, c] => Self::Other([*a, *b, *c]),
                _ => return None
            }
        })
    }
}
