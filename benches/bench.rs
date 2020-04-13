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
        });
    }
    #[bench]
    fn preprocess_iterator(b: &mut Bencher) {
        b.iter(|| {
            let file = open_file("samples/sample1.csv.gz");
            let summary = preprocess_iter(file);
        });
    }
}
