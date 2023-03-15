use std::{io::{Cursor, BufRead}, error::Error};

use json_comments::StripComments;
use serde_json::Value;

const BOM_BYTES: [u8; 3] = [239, 187, 191];
fn strip_bom(b: &[u8]) -> &[u8] {
    if b.len() >= 3 && b[..3] == BOM_BYTES { &b[3..] } else { b }
}

fn find_brackets(b: &[u8]) -> Result<(usize, usize), Box<dyn Error>> {
    let (i, endb) = match b.iter().enumerate().find(|(_, &b)| b == b'{' || b == b'[') {
        Some((i, b'{')) => (i, b'}'),
        Some((i, b'[')) => (i, b']'),
        _ => { return Err(JSONMinifierError)?; }
    };
    let Some(j) = b.iter().rposition(|&b| b == endb) else {
        return Err(JSONMinifierError)?;
    };
    Ok((i, j))
}

const DUMMIES: &[&str] = &["glsl", "html", "js", "kotlin_module", "md", "nbt", "ogg", "txt", "vert", "xml"];

pub fn only_recompress(ftype: &str) -> bool {
    DUMMIES.binary_search(&ftype).is_ok()
}

pub enum MinifyType {
    PNG, JSON, TOML, Hash, Slash
}
impl MinifyType {
    pub fn by_extension(ftype: &str) -> Option<MinifyType> {
        use MinifyType::*;
        match ftype {
            "png" => Some(PNG),
            "json" | "mcmeta" => Some(JSON),
            "toml" => Some(TOML),
            "cfg" | "obj" | "mtl" => Some(Hash),
            "zs" | "fsh" | "vsh" => Some(Slash),
            _ => None
        }
    }

    pub fn minify(&self, v: &Vec<u8>) -> ResultBytes {
        use MinifyType::*;
        match self {
            PNG => minify_png(v),
            JSON => minify_json(v),
            TOML => minify_toml(v),
            Hash => remove_line_comments("#", v),
            Slash => remove_line_comments("//", v)
        }
    }
    pub fn compress_min(&self) -> usize {
        use MinifyType::*;
        match self {
            PNG => 512,
            JSON | TOML => 48,
            _ => 4
        }
    }
}

pub type ResultBytes = Result<Vec<u8>, Box<dyn Error>>;

fn minify_png(v: &Vec<u8>) -> ResultBytes {
    let mut popts = oxipng::Options::default();
    popts.fix_errors = true;
    popts.strip = oxipng::Headers::Safe;
    popts.optimize_alpha = true;
    popts.deflate = oxipng::Deflaters::Libdeflater { compression: 12 };
    //popts.fast_evaluation = false;
    popts.filter.insert(oxipng::RowFilter::Up);
    popts.filter.insert(oxipng::RowFilter::Paeth);
    //popts.filter.insert(oxipng::RowFilter::MinSum);

    Ok(oxipng::optimize_from_memory(&v, &popts)?)
}

fn minify_json(v: &Vec<u8>) -> ResultBytes {
    let mut fv = strip_bom(v);
    let (i, j) = find_brackets(fv)?;
    fv = &fv[i..j+1];
    let strip_comments = StripComments::new(Cursor::new(fv));
    let mut sv: Value = serde_json::from_reader(strip_comments)?;
    if let Value::Object(xm) = &mut sv {
        uncomment_json_recursive(xm)
    }
    Ok(serde_json::to_vec(&sv)?)
}

fn minify_toml(v: &Vec<u8>) -> ResultBytes {
    let fv = std::str::from_utf8(strip_bom(v))?;
    let table: toml::Table = toml::from_str(fv)?;
    Ok(toml::to_string(&table)?
        .lines()
        .map(|l| l.replacen(" = ", "=", 1).into_bytes())
        .collect::<Vec<_>>().join(&b'\n'))
}

fn remove_line_comments(bs: &str, v: &Vec<u8>) -> ResultBytes {
    Ok(v.lines().try_fold(Vec::new(), |mut buf, l| {
        let l = l?;
        if !(l.is_empty() || l.trim_start().starts_with(bs)) {
            buf.extend(l.bytes());
            buf.push(b'\n');
        }
        Ok::<_, std::io::Error>(buf)
    })?)
}

#[derive(Debug)]
pub struct JSONMinifierError;
impl Error for JSONMinifierError {}
impl std::fmt::Display for JSONMinifierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The file has improper opening and/or closing brackets")
    }
}

fn uncomment_json_recursive(m: &mut serde_json::Map<String, Value>) {
    m.retain(|k, _| !k.starts_with('_'));
    m.values_mut().for_each(|v| {
        if let Value::Object(xm) = v {
            uncomment_json_recursive(xm);
        }
    });
}
