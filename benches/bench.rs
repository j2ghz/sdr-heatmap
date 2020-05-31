use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sdr_heatmap::{open_file, preprocess, preprocess_iter, preprocess_par_iter};
use std::io::{Cursor, Read};

fn read_file_to_memory(filename: &str) -> std::boxed::Box<std::io::Cursor<std::vec::Vec<u8>>> {
    let mut buf = Vec::new();
    let mut file = open_file(filename);
    let _ = file.read_to_end(&mut buf);

    Box::new(Cursor::new(buf))
}

fn get_file_size(filename: &str) -> u64 {
    let mut file = open_file(filename);
    let mut length: u64 = 0;
    let mut buf = vec![0; 1024];
    loop {
        let read = file.read(&mut buf).unwrap() as u64;
        length += read;
        if read == 0 {
            break;
        }
    }
    assert_ne!(0, length);
    length
}

fn preprocess_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("preprocess implementations");
    for file in [
        "samples/test0.csv.gz",
        "samples/test1.csv.gz",
        "samples/test2.csv.gz",
        "samples/bench1.csv.gz"
    ]
    .iter()
    {
        let size = get_file_size(file);
        if size > 1000000 {
            group.sample_size(10);
        }
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::new("basic", file), file, |b, file| {
            b.iter_with_large_setup(
                || read_file_to_memory(file),
                |data| {
                    let summary = preprocess(data);
                    black_box(summary);
                },
            )
        });
        group.bench_with_input(BenchmarkId::new("iterator", file), file, |b, file| {
            b.iter_with_large_setup(
                || read_file_to_memory(file),
                |data| {
                    let summary = preprocess_iter(data);
                    black_box(summary);
                },
            )
        });
        group.bench_with_input(BenchmarkId::new("par_iterator", file), file, |b, file| {
            b.iter_with_large_setup(
                || read_file_to_memory(file),
                |data| {
                    let summary = preprocess_par_iter(data);
                    black_box(summary);
                },
            )
        });
    }

    group.finish();
}

criterion_group!(benches, preprocess_bench);
criterion_main!(benches);
