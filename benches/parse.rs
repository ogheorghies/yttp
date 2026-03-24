use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_from_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("from_str");

    group.bench_function("json_strict", |b| {
        b.iter(|| yttp::from_str(black_box(r#"{"g": "https://example.com", "h": {"Authorization": "Bearer tok"}, "1": "j(s b)"}"#)).unwrap())
    });

    group.bench_function("yaml_flow", |b| {
        b.iter(|| yttp::from_str(black_box("{g: https://example.com, h: {a!: tok}, 1: j(s b)}")).unwrap())
    });

    group.bench_function("yaml_block", |b| {
        b.iter(|| yttp::from_str(black_box("g: https://example.com\nh:\n  a!: tok\n1: j(s b)\n")).unwrap())
    });

    group.finish();
}

fn bench_from_json(c: &mut Criterion) {
    c.bench_function("from_json", |b| {
        b.iter(|| yttp::from_json(black_box(r#"{"g": "https://example.com", "h": {"Authorization": "Bearer tok"}}"#)).unwrap())
    });
}

fn bench_from_yaml(c: &mut Criterion) {
    c.bench_function("from_yaml", |b| {
        b.iter(|| yttp::from_yaml(black_box("{g: https://example.com, h: {a!: tok}}")).unwrap())
    });
}

fn bench_expand(c: &mut Criterion) {
    let val = yttp::from_str("{g: https://example.com, h: {a!: bearer!tok, c!: j!}, b: {key: val}}").unwrap();

    c.bench_function("expand", |b| {
        b.iter(|| yttp::expand(black_box(val.clone())).unwrap())
    });
}

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    group.bench_function("json_strict", |b| {
        b.iter(|| yttp::parse(black_box(r#"{"g": "https://example.com", "h": {"Authorization": "Bearer tok"}}"#)).unwrap())
    });

    group.bench_function("yaml_flow", |b| {
        b.iter(|| yttp::parse(black_box("{g: https://example.com, h: {a!: tok}}")).unwrap())
    });

    group.finish();
}

fn bench_expand_headers(c: &mut Criterion) {
    use serde_json::{Map, Value};

    let mut headers = Map::new();
    headers.insert("a!".to_string(), Value::String("bearer!my-token".to_string()));
    headers.insert("c!".to_string(), Value::String("j!".to_string()));
    headers.insert("X-Custom".to_string(), Value::String("value".to_string()));

    c.bench_function("expand_headers", |b| {
        b.iter(|| {
            let mut h = headers.clone();
            yttp::expand_headers(black_box(&mut h));
        })
    });
}

criterion_group!(
    benches,
    bench_from_str,
    bench_from_json,
    bench_from_yaml,
    bench_expand,
    bench_parse,
    bench_expand_headers,
);
criterion_main!(benches);
