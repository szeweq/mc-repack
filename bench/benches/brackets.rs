use criterion::{black_box, criterion_group, criterion_main, Criterion};

const JSON_LIKE_DATA: &[u8] = b"

~ ~ ~  \xfe \xfd \xdd == JSON DATA START: ==
{
    \"a\": 1,
    \"b\": 2.0,
    \"c\": \"c\"
} \xff\xff\xff
== JSON DATA END ==

";

#[inline]
fn find_brackets_direct(b: &[u8]) -> Option<&[u8]> {
    let (i, endb) = match b.iter().enumerate().find(|(_, &b)| b == b'{' || b == b'[') {
        Some((i, b'{')) => (i, b'}'),
        Some((i, b'[')) => (i, b']'),
        _ => { return None; }
    };
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[i..=j])
}

#[inline]
fn find_brackets_new_direct(b: &[u8]) -> Option<&[u8]> {
    let i = b.iter().position(|&b| b == b'{' || b == b'[')?;
    let endb = match b[i] {
        b'{' => b'}',
        b'[' => b']',
        _ => { return None; }
    };
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[i..=j])
}

#[inline]
fn find_brackets_opt(b: &[u8]) -> Option<&[u8]> {
    let (i, endb) = match b.iter().enumerate().find(|(_, &b)| b == b'{' || b == b'[') {
        Some((i, x)) => (i, x ^ 6),
        _ => { return None; }
    };
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[i..=j])
}

#[inline]
fn find_brackets_opt_new(b: &[u8]) -> Option<&[u8]> {
    let i = b.iter().position(|&b| b == b'{' || b == b'[')?;
    let endb = b[i] ^ 6;
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[i..=j])
}

pub fn bench_brackets(c: &mut Criterion) {
    let mut g = c.benchmark_group("brackets");
    g.bench_function("old direct", |b| b.iter(|| find_brackets_direct(black_box(JSON_LIKE_DATA))));
    g.bench_function("new direct", |b| b.iter(|| find_brackets_new_direct(black_box(JSON_LIKE_DATA))));
    g.bench_function("optimized", |b| b.iter(|| find_brackets_opt(black_box(JSON_LIKE_DATA))));
    g.bench_function("optimized new", |b| b.iter(|| find_brackets_opt_new(black_box(JSON_LIKE_DATA))));
    g.finish();
}

criterion_group!(ben, bench_brackets);
criterion_main!(ben);