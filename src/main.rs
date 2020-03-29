use image::png::PNGEncoder;
use std::io;

#[derive(Debug)]
struct Measurement {
    date: String,
    time: String,
    freq_low: u32,
    freq_high: u32,
    freq_step: f64,
    samples: u32,
    values: Vec<f32>,
}

impl Measurement {
    fn get_values(&self) -> Vec<(f64, f32)> {
        self.values
            .iter()
            .zip(0..)
            .map(|(value, i)| ((i as f64) * self.freq_step + (self.freq_low as f64), *value))
            .collect()
    }
}

fn normalize(v: f32) -> Vec<u8> {
    let n = (v + 50.0) * 100.0 / 256.0;
    vec![n as u8, n as u8, 50]
}

fn main() {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(io::stdin());
    let mut date: String = "".to_string();
    let mut time: String = "".to_string();
    let mut batch = Vec::new();
    let mut batchsize = 0;
    let mut img = Vec::new();
    for result in rdr.records() {
        let mut record = result.expect("");
        record.trim();
        assert!(record.len() > 7);
        let m = Measurement {
            date: record.get(0).unwrap().to_string(),
            time: record.get(1).unwrap().to_string(),
            freq_low: record.get(2).unwrap().parse().unwrap(),
            freq_high: record.get(3).unwrap().parse().unwrap(),
            freq_step: record.get(4).unwrap().parse().unwrap(),
            samples: record.get(5).unwrap().parse().unwrap(),
            values: record.iter().skip(6).map(|s| s.parse().unwrap()).collect(),
        };
        if date == m.date && time == m.time {
            let vals = m.get_values();
            img.extend(vals.iter().flat_map(|(_, v)| normalize(*v)));
            batch.push(vals);
        } else {
            println!("{} {}: {:?}", m.date, m.time, batch);
            if batchsize == 0 {
                batchsize = batch.len()
            }
            assert_eq! {batchsize,batch.len()}
            batch.clear();
            date = m.date;
            time = m.time;
        }
    }
    let width = batchsize as u32;
    let height = (img.len() / 3 / batchsize) as u32;
    println!("{}x{}", width, height);
    let f = std::fs::File::create("target/1.png").unwrap();
    PNGEncoder::new(f)
        .encode(&img, width, height, image::ColorType::Rgb8)
        .unwrap();
}
