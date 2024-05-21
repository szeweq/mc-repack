use json_comments::StripComments;
use serde_json::Value;

use crate::cfg::{acfg, ConfigHolder};

use super::{find_brackets, BracketsError, Result_};


acfg!(
    /// A JSON minifier that accepts [`JSONConfig`].
    MinifierJSON: JSONConfig
);
impl ConfigHolder<MinifierJSON> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let (i, j) = find_brackets(b).ok_or(BracketsError)?;
        let fv = &b[i..=j];
        let mut sv: Value = serde_json::from_reader(StripComments::new(fv))?;
        if self.remove_underscored {
            if let Value::Object(xm) = &mut sv {
                uncomment_json_recursive(xm);
            }
        }
        serde_json::to_writer(vout, &sv)?;
        Ok(())
    }
}

/// Configuration for JSON minifier
#[cfg_attr(feature = "serde-cfg", derive(serde::Serialize, serde::Deserialize))]
pub struct JSONConfig {
    /// An optional flag that enables removing underscored keys.
    /// Defaults to `true`.
    pub remove_underscored: bool
}
impl Default for JSONConfig {
    fn default() -> Self {
        Self { remove_underscored: true }
    }
}

fn uncomment_json_recursive(m: &mut serde_json::Map<String, Value>) {
    m.retain(|k, v| {
        if k.starts_with('_') { return false; }
        if let Value::Object(xm) = v {
            uncomment_json_recursive(xm);
        }
        true
    });
}