use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sdr_heatmap::{open_file, preprocess, preprocess_iter, process, process_iter, Palette};
use std::{
    fs::read_dir,
    io::{Cursor, Read},
    path::Path,
};

fn read_file_to_memory(filename: &Path) -> std::boxed::Box<std::io::Cursor<std::vec::Vec<u8>>> {
    let mut buf = Vec::new();
    let mut file = open_file(filename).expect("Can't open file");
    let _ = file.read_to_end(&mut buf).unwrap();

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

fn get_test_files() -> std::vec::Vec<std::path::PathBuf> {
    let dir = read_dir("./samples/").expect("Couldn't read samples directory");
    dir.map(|f| f.unwrap())
        .filter(|f| f.file_name().to_string_lossy().ends_with(".csv.gz"))
        .map(|f| f.path())
        .collect::<Vec<_>>()
}

fn preprocess_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("preprocess implementations");
    for file in get_test_files().iter() {
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
    for file in get_test_files().iter() {
        let size = get_file_size(&file);
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(
            BenchmarkId::new("basic", file.display()),
            &file,
            |b, file| {
                b.iter_with_large_setup(
                    || read_csv_to_memory(file),
                    |data| {
                        let summary = process(data, -1000.0, 1000.0, Palette::Default).unwrap();
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
                        let summary = process_iter(data, -1000.0, 1000.0, 1);
                        black_box(summary);
                    },
                )
            },
        );
    }

    group.finish();
}

criterion_group!(bench, process_bench, preprocess_bench);
criterion_main!(bench);
