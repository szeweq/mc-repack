#![cfg(feature = "png")]

use super::Result_;
use lazy_static::lazy_static;

lazy_static! {
    static ref PNG_OPTS: oxipng::Options = {
        let mut popts = oxipng::Options {
            fix_errors: true,
            strip: oxipng::StripChunks::Safe,
            optimize_alpha: true,
            deflate: oxipng::Deflaters::Libdeflater { compression: 12 },
            ..Default::default()
        };
        popts.filter.insert(oxipng::RowFilter::Up);
        popts.filter.insert(oxipng::RowFilter::Paeth);
        popts
    };
}

pub(super) fn minify_png(v: &[u8], vout: &mut Vec<u8>) -> Result_ {
    let v = oxipng::optimize_from_memory(v, &PNG_OPTS)?;
    let _ = std::mem::replace(vout, v);
    Ok(())
}