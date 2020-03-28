use std::io;

#[derive(Debug)]
struct Measurement {
    date: String,
    time: String,
    freq_low: String,
    freq_high: String,
    freq_step: String,
    samples: String,
    values: Vec<String>,
}
fn main() {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(io::stdin());
    for result in rdr.records() {
        let record = result.expect("");
        assert!(record.len() > 7);
        let m = Measurement {
            date: record.get(0).expect("").to_string(),
            time: record.get(1).expect("").to_string(),
            freq_low: record.get(2).expect("").to_string(),
            freq_high: record.get(3).expect("").to_string(),
            freq_step: record.get(4).expect("").to_string(),
            samples: record.get(5).expect("").to_string(),
            values: record.iter().skip(6).map(|s| s.to_string()).collect(),
        };

        println!("{:?}", m);
    }
}
