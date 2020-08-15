use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sdr_heatmap::{open_file, preprocess, preprocess_iter, process, process_iter};
use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

fn read_file_to_memory(filename: &Path) -> std::boxed::Box<std::io::Cursor<std::vec::Vec<u8>>> {
    let mut buf = Vec::new();
    let mut file = open_file(filename);
    let _ = file.read_to_end(&mut buf);

    Box::new(Cursor::new(buf))
}

fn read_csv_to_memory(
    filename: &Path,
) -> csv::Reader<std::boxed::Box<std::io::Cursor<std::vec::Vec<u8>>>> {
    let file = read_file_to_memory(filename);
    csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file)
}

fn get_file_size(filename: &Path) -> u64 {
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
    let files = [
        "samples/test0.csv.gz",
        "samples/test1.csv.gz",
        "samples/test2.csv.gz",
        "samples/test3.csv.gz",
        "samples/test4.csv.gz",
        "samples/test5.csv.gz",
        "samples/bench1.csv.gz",
    ]
    .iter()
    .map(PathBuf::from)
    .collect::<Vec<_>>();
    for file in files.iter() {
        let size = get_file_size(&file);
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(
            BenchmarkId::new("basic", file.display()),
            &file,
            |b, file| {
                b.iter_with_large_setup(
                    || read_file_to_memory(file),
                    |data| {
                        let summary = preprocess(data);
                        black_box(summary);
                    },
                )
            },
        );
        group.bench_with_input(
            BenchmarkId::new("iterator", file.display()),
            &file,
            |b, file| {
                b.iter_with_large_setup(
                    || read_file_to_memory(file),
                    |data| {
                        let summary = preprocess_iter(data);
                        black_box(summary);
                    },
                )
            },
        );
    }

    group.finish();
}

fn process_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("process implementations");
    let files = [
        "samples/test0.csv.gz",
        "samples/test1.csv.gz",
        "samples/test2.csv.gz",
        "samples/test3.csv.gz",
        "samples/test4.csv.gz",
        "samples/test5.csv.gz",
        "samples/bench1.csv.gz",
    ]
    .iter()
    .map(PathBuf::from)
    .collect::<Vec<_>>();
    for file in files.iter() {
        let size = get_file_size(&file);
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(
            BenchmarkId::new("basic", file.display()),
            &file,
            |b, file| {
                b.iter_with_large_setup(
                    || read_csv_to_memory(file),
                    |data| {
                        let summary = process(data, -1000.0, 1000.0);
                        black_box(summary);
                    },
                )
            },
        );
        group.bench_with_input(
            BenchmarkId::new("iterator", file.display()),
            &file,
            |b, file| {
                b.iter_with_large_setup(
                    || read_csv_to_memory(file),
                    |data| {
                        let summary = process_iter(data, -1000.0, 1000.0);
                        black_box(summary);
                    },
                )
            },
        );
    }

    group.finish();
}

criterion_group!(bench, process_bench); //, preprocess_bench);
criterion_main!(bench);
