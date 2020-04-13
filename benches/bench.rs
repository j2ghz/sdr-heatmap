#![cfg_attr(feature = "unstable", feature(test))]

#[cfg(all(feature = "unstable", test))]
mod bench {

    extern crate test;

    use sdr_heatmap::*;
    use test::Bencher;

    #[bench]
    fn preprocess_basic(b: &mut Bencher) {
        b.iter(|| {
            let file = open_file("samples/sample1.csv.gz");
            let summary = preprocess(file);
            println!("{} {}", summary.min, summary.max);
        });
    }
    #[bench]
    fn preprocess_iter(b: &mut Bencher) {
        b.iter(|| {
            let file = open_file("samples/sample1.csv.gz");
            let summary = preprocess_iter(file);
            println!("{} {}", summary.min, summary.max);
        });
    }
}
