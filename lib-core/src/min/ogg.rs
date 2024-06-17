#![cfg(feature = "ogg")]
use std::io::Cursor;

use optivorbis::{remuxer::ogg_to_ogg::Settings, OggToOgg, Remuxer, VorbisCommentFieldsAction, VorbisOptimizerSettings, VorbisVendorStringAction};

use crate::cfg::{acfg, ConfigHolder};
use super::Result_;

acfg!(
    /// A OGG minifier that accepts [`OGGConfig`].
    MinifierOGG: OGGConfig
);
impl ConfigHolder<MinifierOGG> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        OggToOgg::new(Settings::default(), self.ogg_opts()).remux(&mut Cursor::new(b), vout)?;
        Ok(())
    }
}


/// Configuration for OGG minifier
#[cfg_attr(feature = "serde-cfg", derive(serde::Serialize, serde::Deserialize))]
pub struct OGGConfig {
    /// An optional flag that enables removing comments.
    /// Defaults to `true`.
    pub remove_comments: bool
}
impl Default for OGGConfig {
    fn default() -> Self {
        Self { remove_comments: true }
    }
}
impl OGGConfig {
    #[inline]
    fn ogg_opts(&self) -> VorbisOptimizerSettings {
        let mut opts = VorbisOptimizerSettings::default();
        opts.comment_fields_action = if self.remove_comments {
            VorbisCommentFieldsAction::Delete
        } else {
            VorbisCommentFieldsAction::Copy
        };
        opts.vendor_string_action = if self.remove_comments {
            VorbisVendorStringAction::Empty
        } else {
            VorbisVendorStringAction::Copy
        };
        opts
    }
}