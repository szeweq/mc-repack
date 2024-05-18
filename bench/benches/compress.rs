use std::{io, num::NonZeroU64};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

struct CounterWrite(usize);
impl io::Write for CounterWrite {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = buf.len();
        self.0 += n;
        Ok(n)
    }
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn calc_entropy(b: &[u8]) -> f64 {
    if b.is_empty() { return 0.0; }
    let mut freq = [0usize; 256];
    for &b in b { freq[b as usize] += 1; }
    let total = b.len() as f64;
    let logt = total.log2();
    let e = freq.into_iter().filter(|&f| f != 0)
        .map(|f| -(f as f64) * ((f as f64).log2() - logt))
        .sum::<f64>() / total;
    assert!((0.0..=8.0).contains(&e), "Invalid entropy: {}", e);
    e
}

fn calc_entropy_f32(b: &[u8]) -> f32 {
    if b.is_empty() { return 0.0; }
    let mut freq = [0usize; 256];
    for &b in b { freq[b as usize] += 1; }
    let total = b.len() as f32;
    let logt = total.log2();
    let e = freq.into_iter().filter(|&f| f != 0)
        .map(|f| -(f as f32) * ((f as f32).log2() - logt))
        .sum::<f32>() / total;
    assert!((0.0..=8.0).contains(&e), "Invalid entropy: {}", e);
    e
}

pub fn compressed_len_z(b: &[u8], opts: zopfli::Options) -> usize {
    use zopfli::{DeflateEncoder, BlockType};
    let mut enc = DeflateEncoder::new(opts, BlockType::Dynamic, CounterWrite(0));
    io::copy(&mut &*b, &mut enc).unwrap();
    enc.finish().unwrap().0
}

pub fn compressed_len_d(b: &[u8]) -> usize {
    use flate2::{Compression, write::DeflateEncoder};
    let mut enc = DeflateEncoder::new(CounterWrite(0), Compression::best());
    io::copy(&mut &*b, &mut enc).unwrap();
    enc.finish().unwrap().0
}

pub fn compressed_len_d_new(b: &[u8]) -> usize {
    use flate2::{Compression, write::DeflateEncoder};
    use std::io::Write;
    let mut enc = DeflateEncoder::new(CounterWrite(0), Compression::best());
    enc.write_all(b).unwrap();
    enc.finish().unwrap().0
}

pub fn compressed_len_d_sink(b: &[u8]) -> usize {
    use flate2::{Compression, write::DeflateEncoder};
    use std::io::Write;
    let mut enc = DeflateEncoder::new(io::sink(), Compression::best());
    enc.write_all(b).unwrap();
    enc.try_finish().unwrap();
    enc.total_out() as usize
}

pub fn compressed_len_d_legacy(b: &[u8]) -> usize {
    use flate2::{Compression, bufread::DeflateEncoder};
    use std::io::Read;
    let enc = DeflateEncoder::new(b, Compression::best());
    enc.bytes().count()
}

fn check_len(b: &[u8]) {
    let c1 = compressed_len_d(b);
    let c2 = compressed_len_d_legacy(b);
    assert_eq!(c1, c2, "Optimal ({}) != Legacy ({})", c1, c2);
    let c3 = compressed_len_d_new(b);
    assert_eq!(c1, c3, "Optimal ({}) != New ({})", c1, c3);
    let c4 = compressed_len_d_sink(b);
    assert_eq!(c1, c4, "Optimal ({}) != Sink ({})", c1, c4);
}

pub fn zopfli_opts(it: u64) -> zopfli::Options {
    zopfli::Options {
        iteration_count: NonZeroU64::new(it).unwrap(),
        iterations_without_improvement: NonZeroU64::new(6).unwrap(),
        ..Default::default()
    }
}

pub fn bench_group_compress(c: &mut Criterion, name: &str, data: &[u8]) {
    let gtb = Throughput::Bytes(data.len() as u64);
    let mut g = c.benchmark_group(format!("entropy {name}"));
    g.throughput(gtb.clone());
    g.bench_function("f64", |b| b.iter(|| calc_entropy(data)));
    g.bench_function("f32", |b| b.iter(|| calc_entropy_f32(data)));
    g.finish();
    g = c.benchmark_group(format!("deflate {name}"));
    g.throughput(gtb.clone());
    g.bench_function("optimal", |b| b.iter(|| compressed_len_d(data)));
    g.bench_function("new", |b| b.iter(|| compressed_len_d_new(data)));
    g.bench_function("sink", |b| b.iter(|| compressed_len_d_sink(data)));
    g.bench_function("legacy", |b| b.iter(|| compressed_len_d_legacy(data)));
    g.finish();
    g = c.benchmark_group(format!("zopfli {name}"));
    g.throughput(gtb);
    for i in [1, 5, 10, 15] {
        g.bench_with_input(BenchmarkId::new("iter", i), &zopfli_opts(1), |b, o| b.iter(|| compressed_len_z(data, *o)));
    }
    g.finish();
}

pub fn bench_png_compress_rate(c: &mut Criterion) {
    let readme_data = std::fs::read("../README.md").unwrap();
    let logo_data = std::fs::read("../mc-repack-logo.png").unwrap();
    check_len(&readme_data);
    bench_group_compress(c, "png", &logo_data);
    bench_group_compress(c, "readme", &readme_data);
}

criterion_group!(ben, bench_png_compress_rate);
criterion_main!(ben);
