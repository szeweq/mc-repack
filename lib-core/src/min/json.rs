use std::io::Cursor;

use json_comments::StripComments;
use serde_json::Value;

use super::{find_brackets, BracketsError, Result_};

pub(super) fn minify_json(v: &[u8], vout: &mut Vec<u8>) -> Result_ {
    let (i, j) = find_brackets(v).ok_or(BracketsError)?;
    let fv = &v[i..=j];
    let strip_comments = StripComments::new(Cursor::new(fv));
    let mut sv: Value = serde_json::from_reader(strip_comments)?;
    if let Value::Object(xm) = &mut sv {
        uncomment_json_recursive(xm);
    }
    serde_json::to_writer(vout, &sv)?;
    Ok(())
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