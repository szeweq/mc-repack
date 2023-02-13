mod minify;

use std::{fs::File, io::{self, Write, Read}};

use flate2::read::DeflateEncoder;
use indicatif::{ProgressBar, ProgressStyle};
use zip::{ZipArchive, ZipWriter, write::FileOptions, CompressionMethod};

use crate::minify::*;

fn main() -> io::Result<()> {
    println!("MC REPACK!");
    let mut pngc = PNGMinifier { opts: oxipng::Options::default() };
    pngc.opts.fix_errors = true;

    let fopts = FileOptions::default().compression_level(Some(9));

    let mut oldjar = ZipArchive::new(File::open("mod.jar")?)?;
    let mut newjar = ZipWriter::new(File::create("mod_new.jar")?);
    let mut dsum = 0;
    let jfc = oldjar.len() as u64;
    let pb = ProgressBar::new(jfc).with_style(
        ProgressStyle::with_template("{bar} {pos}/{len} {wide_msg}").unwrap()
    );


    for i in 0..jfc {
        let mut jf = oldjar.by_index(i as usize)?;
        let fname = jf.name().to_string();
        pb.set_position(i);
        pb.set_message(fname.clone());
        if jf.is_dir() {
            newjar.raw_copy_file(jf)?;
            continue;
        }
        let comp: &dyn Minifier = if fname.ends_with(".json") || fname.ends_with(".mcmeta") {
            &JSONMinifier
        } else if fname.ends_with(".png") {
            &pngc
        } else {
            newjar.raw_copy_file(jf)?;
            continue;
        };
        let fsz = jf.size() as i64;
        let (buf, c) = match comp.minify(&mut jf) {
            Ok(x) => x,
            Err(e) => {
                println!("{}: {}", fname, e);
                newjar.raw_copy_file(jf)?;
                continue;
            }
        };
        dsum -= (buf.len() as i64) - fsz;
        newjar.start_file(&fname, fopts.clone()
            .compression_method(aggressive_check(&buf, c)?)
        )?;
        newjar.write_all(&buf)?;
    }

    pb.finish_with_message("Finished!");
    newjar.finish()?;
    println!("[REPACK] Bytes saved: {}", dsum);

    Ok(())
}

fn aggressive_check(b: &[u8], c: bool) -> io::Result<CompressionMethod> {
    let nc = if !c {
        let de = DeflateEncoder::new(b, flate2::Compression::best());
        let sum = de.bytes().count();
        sum < b.len()
    } else { c };
    Ok(if nc { CompressionMethod::Deflated } else { CompressionMethod::Stored })
}
