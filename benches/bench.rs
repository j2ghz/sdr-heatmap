use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use sdr_heatmap::{open_file, preprocess, preprocess_iter};
use std::{
    io::{Cursor, Read},
    time::Duration,
};

fn read_file_to_memory() -> std::boxed::Box<std::io::Cursor<std::vec::Vec<u8>>> {
    let mut buf = Vec::new();
    let mut file = open_file("samples/sample1.csv.gz");
    let _ = file.read_to_end(&mut buf);

    Box::new(Cursor::new(buf))
}

fn preprocess_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("preprocess implementations");

    group.throughput(Throughput::Bytes(46474948));
    group.measurement_time(Duration::from_secs(60));
    group.sample_size(10);

    group.bench_function("basic", |b| {
        b.iter_with_large_setup(read_file_to_memory, |data| {
            let summary = preprocess(data);
            black_box(summary);
        })
    });
    group.bench_function("iterator", |b| {
        b.iter_with_large_setup(read_file_to_memory, |data| {
            let summary = preprocess_iter(data);
            black_box(summary);
        })
    });
    group.finish();
}

criterion_group!(benches, preprocess_bench);
criterion_main!(benches);
