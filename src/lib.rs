use csv::StringRecord;
use flate2::read::GzDecoder;
use image::png::PNGEncoder;
use log::*;
use quantiles::ckms::CKMS;
use std::f32;
use std::io::prelude::*;
use std::{cmp::Ordering, fs::File};

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
            freq_low: parse(record.get(2).unwrap()).unwrap(),
            freq_high: parse(record.get(3).unwrap()).unwrap(),
            freq_step: parse(record.get(4).unwrap()).unwrap(),
            samples: parse(record.get(5).unwrap()).unwrap(),
            values,
        }
    }
}

fn parse<T: std::str::FromStr>(
    string: &str,
) -> std::result::Result<T, <T as std::str::FromStr>::Err> {
    let parsed = string.parse::<T>();
    debug_assert!(parsed.is_ok(), "Could not parse {}", string);
    parsed
}

/// Places value on a scale from min to max, and transforms it to an integer scale from 0 to 255. Returns a color using the default palette.
pub fn scale_tocolor(value: f32, min: f32, max: f32) -> Vec<u8> {
    debug_assert!(
        value >= min || value == f32::NEG_INFINITY,
        "Value {} is smaller than min {}",
        value,
        min
    );
    debug_assert!(
        value <= max || value == f32::INFINITY,
        "Value {} is greater than max {}",
        value,
        max
    );
    if value < min {
        return vec![0, 0, 0];
    } else if value > max {
        return vec![255, 255, 255];
    } else if value == max {
        return vec![255, 255, 50];
    } else if value == min {
        return vec![0, 0, 50];
    }
    let scaled = (value - min) * (255.0 / (max - min));
    if scaled < 0.0 || scaled > 255.0 {
        warn!("Computed invalid color! Value range: {} to {}, Value: {}, Color range: 0-255, Color: {}", min,max,value,scaled)
    }
    debug_assert!(
        scaled >= 0.0,
        "Scaled value is outside of range: {}",
        scaled
    );
    debug_assert!(
        scaled <= 255.0,
        "Scaled value is outside of range: {}",
        scaled
    );
    vec![scaled as u8, scaled as u8, 50]
}

pub fn open_file(path: &str) -> Box<dyn std::io::Read> {
    let file = File::open(path).unwrap();
    if path.ends_with(".gz") {
        Box::new(GzDecoder::new(file))
    } else {
        Box::new(file)
    }
}

fn read_file<T: std::io::Read>(file: T) -> csv::Reader<T> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file)
}

pub fn main(path: &str) {
    info!("Loading: {}", path);
    //Preprocess
    let file = open_file(path);
    let summary = preprocess(file);
    info!("Color values {} to {}", summary.min, summary.max);
    //Process
    let file = open_file(path);
    let reader = read_file(file);
    let (datawidth, dataheight, img) = process(reader, summary.min, summary.max);
    //Draw
    let (height, imgdata) = create_image(datawidth, dataheight, img);
    let dest = path.to_owned() + ".png";
    save_image(datawidth, height, imgdata, &dest).unwrap();
}

pub fn preprocess(file: Box<dyn Read>) -> Summary {
    let reader = read_file(file);
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for result in reader.into_records() {
        let mut record = result.unwrap();
        record.trim();
        let values: Vec<f32> = record
            .iter()
            .skip(6)
            .map(|s| {
                if s == "-nan" {
                    f32::NAN
                } else {
                    s.parse::<f32>()
                        .unwrap_or_else(|e| panic!("{} should be a valid float: {:?}", s, e))
                }
            })
            .collect();
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
    Summary { min, max }
}

#[derive(PartialEq, Debug)]
pub struct Summary {
    pub min: f32,
    pub max: f32,
}

impl Summary {
    fn empty() -> Self {
        Self {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
        }
    }
}

pub fn preprocess_iter(file: Box<dyn Read>) -> Summary {
    read_file(file)
        .into_records()
        .filter_map(|x| match x {
            Ok(line) => Some(line),
            Err(e) => {
                warn!("Error reading a line from the csv: {}", e);
                None
            }
        })
        .flat_map(|line| {
            line.into_iter()
                .skip(6)
                .map(|s| {
                    if s == "-nan" {
                        f32::NAN
                    } else {
                        s.trim().parse::<f32>().unwrap_or_else(|e| {
                            panic!("'{}' should be a valid float: '{:?}'", s, e)
                        })
                    }
                })
                .collect::<Vec<f32>>()
        })
        .fold(Summary::empty(), |s, i| Summary {
            min: if i.is_finite() { s.min.min(i) } else { s.min },
            max: if i.is_finite() { s.max.max(i) } else { s.max },
        })
}

pub fn preprocess_ckms(file: Box<dyn Read>) -> Summary {
    let values = read_file(file)
        .into_records()
        .filter_map(|x| match x {
            Ok(line) => Some(line),
            Err(e) => {
                warn!("Error reading a line from the csv: {}", e);
                None
            }
        })
        .flat_map(|line| {
            line.into_iter()
                .skip(6)
                .map(|s| {
                    if s == "-nan" {
                        f32::NAN
                    } else {
                        s.trim().parse::<f32>().unwrap_or_else(|e| {
                            panic!("'{}' should be a valid float: '{:?}'", s, e)
                        })
                    }
                })
                .collect::<Vec<f32>>()
        });
    let mut ckms = CKMS::<f32>::new(0.1);
    for i in values {
        ckms.insert(i);
    }
    let min = ckms.query(0.01);
    let max = ckms.query(0.99);
    println!("min: {:?} max: {:?}", min, max);
    Summary {
        min: min.unwrap().1,
        max: max.unwrap().1,
    }
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
        img.extend(vals.iter().flat_map(|(_, v)| scale_tocolor(*v, min, max)));
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
    match expected_length.cmp(&imgdata.len()) {
        Ordering::Greater => {
            warn!("Image is missing some values, was the file cut early? Filling black.",);
            imgdata.append(&mut vec![0; expected_length - imgdata.len()]);
        }
        Ordering::Less => {
            warn!("Image has too many values, was the file cut early? Trimming.",);
            imgdata.truncate(expected_length);
        }
        Ordering::Equal => {}
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
    use crate::*;
    #[test]
    fn normalize_goes_up() {
        assert_eq!(
            (0..255)
                .map(|v| v as f32)
                .map(|v| scale_tocolor(v, 0.0, 255.0).first().cloned().unwrap())
                .collect::<Vec<_>>(),
            (0..255).map(|v| v as u8).collect::<Vec<_>>()
        );
    }

    #[test]
    fn normalize_max() {
        assert_eq!(scale_tocolor(23.02, -29.4, 23.02), vec![255, 255, 50]);
    }

    #[test]
    fn preprocess_result() {
        let res = preprocess(open_file("samples/sample1.csv.gz"));
        assert_eq!(
            res,
            Summary {
                min: -29.4,
                max: 21.35
            }
        );
    }

    #[test]
    fn preprocess_iter_result() {
        let res = preprocess_iter(open_file("samples/sample1.csv.gz"));
        assert_eq!(
            res,
            Summary {
                min: -29.4,
                max: 21.35
            }
        );
    }

    #[test]
    fn preprocess_ckms_result() {
        let res = preprocess_ckms(open_file("samples/sample1.csv.gz"));
        assert_eq!(
            res,
            Summary {
                min: -29.4,
                max: 21.35
            }
        );
    }

    #[test]
    fn complete() {
        main("samples/sample1.csv.gz")
    }
}
