use csv::StringRecord;
use flate2::read::GzDecoder;
use image::png::PNGEncoder;
use log::*;
use std::f32;
use std::fs::File;

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
            .map(|(value, i)| {
                (
                    ((i) as f64) * self.freq_step + (self.freq_low as f64),
                    *value,
                )
            })
            .collect()
    }
    fn new(record: StringRecord) -> Measurement {
        let mut values: Vec<_> = record.iter().skip(6).map(|s| s.parse().unwrap()).collect();
        values.truncate(record.len() - 7);
        Measurement {
            date: record.get(0).unwrap().to_string(),
            time: record.get(1).unwrap().to_string(),
            freq_low: record.get(2).unwrap().parse().unwrap(),
            freq_high: record.get(3).unwrap().parse().unwrap(),
            freq_step: record.get(4).unwrap().parse().unwrap(),
            samples: record.get(5).unwrap().parse().unwrap(),
            values,
        }
    }
}

pub fn normalize(v: f32, min: f32, max: f32) -> Vec<u8> {
    debug_assert!(v >= min || v == f32::NEG_INFINITY);
    debug_assert!(v <= max || v == f32::INFINITY);
    if v < min {
        return vec![0, 0, 0];
    }
    if v > max {
        return vec![255, 255, 255];
    }
    let n = (v - min) * (255.0 / (max - min));
    debug_assert!(n >= 0.0);
    debug_assert!(n <= 255.0);
    vec![n as u8, n as u8, 50]
}

fn read_file(path: &str) -> csv::Reader<Box<dyn std::io::Read>> {
    let file = File::open(path).unwrap();
    if path.ends_with(".gz") {
        let decomp = GzDecoder::new(file);
        csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(Box::new(decomp))
    } else {
        csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(Box::new(file))
    }
}

pub fn main(path: &str) {
    info!("Loading: {}", path);

    let reader = read_file(path);
    let (min, max) = preprocess(reader);
    let reader = read_file(path);
    let (datawidth, dataheight, img) = process(reader, min, max);
    let (height, imgdata) = create_image(datawidth, dataheight, img);
    let dest = path.to_owned() + ".png";
    save_image(datawidth, height, imgdata, &dest).unwrap();
}

fn preprocess(reader: csv::Reader<Box<dyn std::io::Read>>) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for result in reader.into_records() {
        let mut record = result.unwrap();
        record.trim();
        let values: Vec<f32> = record.iter().skip(6).map(|s| s.parse().unwrap()).collect();
        for value in values {
            if value != f32::INFINITY && value != f32::NEG_INFINITY {
                if value > max {
                    max = value
                }
                if value < min {
                    min = value
                }
            }
        }
    }
    info!("Color values {} to {}", min, max);
    (min, max)
}

fn process(
    reader: csv::Reader<Box<dyn std::io::Read>>,
    min: f32,
    max: f32,
) -> (usize, usize, std::vec::Vec<u8>) {
    let mut date: String = "".to_string();
    let mut time: String = "".to_string();
    let mut batch = Vec::new();
    let mut datawidth = 0;
    let mut img = Vec::new();
    for result in reader.into_records() {
        let mut record = result.unwrap();
        record.trim();
        assert!(record.len() > 7);
        let m = Measurement::new(record);
        let vals = m.get_values();
        if date == m.date && time == m.time {
        } else {
            if datawidth == 0 {
                datawidth = batch.len()
            }
            debug_assert_eq! {datawidth,batch.len()}
            batch.clear();
            date = m.date;
            time = m.time;
        }
        img.extend(vals.iter().flat_map(|(_, v)| normalize(*v, min, max)));
        batch.extend(vals);
    }
    if datawidth == 0 {
        datawidth = batch.len()
    }
    info!("Img data {}x{}", datawidth, batch.len());
    (datawidth, img.len() / 3 / datawidth, img)
}

fn tape_measure(width: usize, imgdata: &mut Vec<u8>) {
    let length = width * 26 * 3;
    imgdata.append(&mut vec![0; length]);
}

fn create_image(width: usize, height: usize, mut img: Vec<u8>) -> (usize, std::vec::Vec<u8>) {
    info!("Raw {}x{}", width, height);
    let mut imgdata: Vec<u8> = Vec::new();
    tape_measure(width, &mut imgdata);
    imgdata.append(&mut img);
    let height = height + 26;
    let expected_length = width * height * 3;
    if expected_length > imgdata.len() {
        warn!("Image is missing some values, was the files cut early? Filling black.",);
        imgdata.append(&mut vec![0; expected_length - imgdata.len()]);
    } else if expected_length < imgdata.len() {
        warn!("Image has too many values, was the files cut early? Trimming.",);
        imgdata.truncate(expected_length);
    }
    (height, imgdata)
}

fn save_image(
    width: usize,
    height: usize,
    imgdata: Vec<u8>,
    dest: &str,
) -> std::result::Result<(), image::error::ImageError> {
    info!("Saving {} {}x{}", dest, width, height);
    let f = std::fs::File::create(dest).unwrap();
    PNGEncoder::new(f).encode(
        &imgdata,
        width as u32,
        height as u32,
        image::ColorType::Rgb8,
    )
}

#[cfg(test)]
mod tests {
    use crate::normalize;
    #[test]
    fn normalize_goes_up() {
        assert_eq!(
            (0..255)
                .map(|v| v as f32)
                .map(|v| normalize(v, 0.0, 255.0).first().cloned().unwrap())
                .collect::<Vec<_>>(),
            (0..255).map(|v| v as u8).collect::<Vec<_>>()
        );
    }
}
