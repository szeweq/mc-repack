use criterion::{black_box, criterion_group, criterion_main, Criterion};

const JSON_LIKE_DATA: &[u8] = b"
 \t The benchmark content begins below:
~ ~ ~  \xfe \xfd \xdd == BEGINNING JSON CONTENT ==
{
    \"a\": 1,
    \"b\": 2.0,
    \"c\": \"c\"
} \xff\xff\xff
== ENDING JSON CONTENT ==
 \t The benchmark content ends above.
";

const SIMPLE_DATA: &[u8] = b"
{\"a\": 1, \"b\": 2.0, \"c\": \"c\"}
";

#[inline]
fn find_brackets(b: &[u8]) -> Option<&[u8]> {
    let (i, endb) = match b.iter().enumerate().find(|(_, &b)| matches!(b, b'{' | b'[')) {
        Some((i, b'{')) => (i, b'}'),
        Some((i, b'[')) => (i, b']'),
        _ => { return None; }
    };
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[i..=j])
}

#[inline]
fn find_brackets_new(b: &[u8]) -> Option<&[u8]> {
    let i = b.iter().position(|&b| matches!(b, b'{' | b'['))?;
    let endb = match b[i] {
        b'{' => b'}',
        b'[' => b']',
        _ => { return None; }
    };
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[i..=j])
}

#[inline]
fn find_brackets_new_2(b: &[u8]) -> Option<&[u8]> {
    let i = b.iter().position(|&b| matches!(b, b'{' | b'['))?;
    let endb = match b[i] {
        b'{' => b'}',
        b'[' => b']',
        _ => { unreachable!() }
    };
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[i..=j])
}

#[inline]
fn find_brackets_opt(b: &[u8]) -> Option<&[u8]> {
    let (i, endb) = match b.iter().enumerate().find(|(_, &b)| matches!(b, b'{' | b'[')) {
        Some((i, x)) => (i, x ^ 6),
        _ => { return None; }
    };
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[i..=j])
}

#[inline]
fn find_brackets_opt_new(b: &[u8]) -> Option<&[u8]> {
    let i = b.iter().position(|&b| matches!(b, b'{' | b'['))?;
    let endb = b[i] ^ 6;
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[i..=j])
}

#[inline]
fn find_brackets_mut(mut b: &[u8]) -> Option<&[u8]> {
    let i = b.iter().position(|&b| matches!(b, b'{' | b'['))?;
    b = &b[i..];
    let endb = b[0] ^ 6;
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[..=j])
}

fn make_bench_with(c: &mut Criterion, name: &str, data: &'static [u8]) {
    let mut g = c.benchmark_group(name);
    g.bench_with_input("mut", data, |b, d| b.iter(|| find_brackets_mut(black_box(d))));
    g.bench_with_input("old direct", data, |b, d| b.iter(|| find_brackets(black_box(d))));
    g.bench_with_input("new direct", data, |b, d| b.iter(|| find_brackets_new(black_box(d))));
    g.bench_with_input("new direct 2", data, |b, d| b.iter(|| find_brackets_new_2(black_box(d))));
    g.bench_with_input("optimized", data, |b, d| b.iter(|| find_brackets_opt(black_box(d))));
    g.bench_with_input("optimized new", data, |b, d| b.iter(|| find_brackets_opt_new(black_box(d))));
    g.finish();
}

pub fn bench_brackets(c: &mut Criterion) {
    make_bench_with(c, "brackets", JSON_LIKE_DATA);
}

pub fn bench_brackets_simple(c: &mut Criterion) {
    make_bench_with(c, "brackets simple", SIMPLE_DATA);
}

criterion_group!(ben, bench_brackets, bench_brackets_simple);
criterion_main!(ben);