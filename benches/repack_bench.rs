use std::io::Read;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flate2::bufread::DeflateEncoder;

pub fn compress_check(b: &[u8], compress_min: usize) -> bool {
    let lb = b.len();
    if lb > compress_min {
        let de = DeflateEncoder::new(b, flate2::Compression::best());
        let sum = de.bytes().count();
        sum < lb
    } else { false }
}

const CCHECK: &[u8] = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Pellentesque placerat auctor eros sed eget.";

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("compress check", |b| {
        b.iter(|| {
            compress_check(black_box(CCHECK), 0)
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);