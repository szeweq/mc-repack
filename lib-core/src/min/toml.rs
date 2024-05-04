#![cfg(feature = "toml")]

use super::Result_;

pub(super) fn minify_toml(v: &[u8], vout: &mut Vec<u8>) -> Result_ {
    let fv = std::str::from_utf8(v)?;
    let mut table: toml::Table = toml::from_str(fv)?;
    strip_toml_table(&mut table);
    toml::to_string(&table)?.lines().for_each(|l| {
        match l.split_once(" = ") {
            Some((k, v)) => {
                vout.extend_from_slice(k.as_bytes());
                vout.push(b'=');
                vout.extend_from_slice(v.as_bytes());
            }
            None => vout.extend_from_slice(l.as_bytes()),
        }
        vout.push(b'\n');
    });
    Ok(())
}

fn strip_toml_table(t: &mut toml::Table) {
    for (_, v) in t {
        strip_toml_value(v);
    }
}
fn strip_toml_array(a: &mut Vec<toml::Value>) {
    for v in a {
        strip_toml_value(v);
    }
}
fn strip_toml_value(v: &mut toml::Value) {
    match v {
        toml::Value::Table(st) => { strip_toml_table(st); }
        toml::Value::String(s) => { super::strip_string(s); }
        toml::Value::Array(a) => { strip_toml_array(a); }
        _ => {}
    }
}