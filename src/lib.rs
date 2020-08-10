use csv::StringRecord;
use flate2::read::GzDecoder;
use image::png::PNGEncoder;
use log::*;
use rayon::prelude::*;
use std::f32;
use std::io::prelude::*;
use std::path::Path;
use std::{cmp::Ordering, fs::File};
pub mod colors;
use colors::scale_tocolor;
use colors::Palettes;

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

    fn combine_f32(s: Self, i: f32) -> Self {
        Summary::combine(s, Summary { min: i, max: i })
    }

    fn combine(a: Self, b: Self) -> Self {
        Self {
            min: {
                let a = a.min;
                let b = b.min;
                if a.is_finite() {
                    a.min(b)
                } else {
                    b
                }
            },
            max: {
                let a = a.max;
                let b = b.max;
                if a.is_finite() {
                    a.max(b)
                } else {
                    b
                }
            },
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

pub fn open_file(path: &Path) -> Box<dyn std::io::Read> {
    let file = File::open(path).unwrap();
    if path.extension().unwrap() == "gz" {
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

pub fn main(path: &Path) {
    info!("Loading: {}", path.display());
    //Preprocess
    let file = open_file(path);
    let summary = preprocess_par_iter(file);
    info!("Color values {} to {}", summary.min, summary.max);
    //Process
    let file = open_file(path);
    let reader = read_file(file);
    let (datawidth, dataheight, img) = process(reader, summary.min, summary.max);
    //Draw
    let (height, imgdata) = create_image(datawidth, dataheight, img);
    let dest = path.with_extension("png");
    save_image(datawidth, height, imgdata, dest.to_str().unwrap()).unwrap();
}

pub fn preprocess(file: Box<dyn Read>) -> Summary {
    let reader = read_file(file);
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for result in reader.into_records() {
        let record = {
            let mut x = result.unwrap();
            x.trim();
            x
        };
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

pub fn preprocess_iter(file: Box<dyn Read>) -> Summary {
    read_file(file)
        .into_records()
        .map(|x| x.unwrap())
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
        .fold(Summary::empty(), Summary::combine_f32)
}

pub fn preprocess_par_iter(file: Box<dyn Read>) -> Summary {
    read_file(file)
        .into_records()
        .collect::<Vec<_>>()
        .into_iter()
        .par_bridge()
        .map(|x| x.unwrap())
        .flat_map(|line| {
            line.into_iter()
                .skip(6)
                .map(|s| s.to_owned())
                .collect::<Vec<String>>()
        })
        .map(|s| {
            if s == "-nan" {
                f32::NAN
            } else {
                s.trim()
                    .parse::<f32>()
                    .unwrap_or_else(|e| panic!("'{}' should be a valid float: '{:?}'", s, e))
            }
        })
        .fold(Summary::empty, Summary::combine_f32)
        .reduce(Summary::empty, Summary::combine)
}

fn process(
    reader: csv::Reader<Box<dyn std::io::Read>>,
    min: f32,
    max: f32,
) -> (usize, usize, std::vec::Vec<u8>) {
    let mut date: String = "".to_string();
    let mut time: String = "".to_string();
    let mut batch = 0;
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
                datawidth = batch;
            }
            debug_assert_eq! {datawidth,batch}
            batch = 0;
            date = m.date;
            time = m.time;
        }
        for (_, v) in vals {
            let pixel = scale_tocolor(Palettes::Default, v, min, max);
            img.extend(pixel.iter());
            batch += 1;
        }
    }
    if datawidth == 0 {
        datawidth = batch;
    }
    info!("Img data {}x{}", datawidth, batch);
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
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn preprocess_result() {
        let res = preprocess(open_file(Path::new("samples/sample1.csv.gz")));
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
        let res = preprocess_iter(open_file(Path::new("samples/sample1.csv.gz")));
        assert_eq!(
            res,
            Summary {
                min: -29.4,
                max: 21.35
            }
        );
    }

    #[test]
    fn preprocess_par_iter_result() {
        let res = preprocess_par_iter(open_file(Path::new("samples/sample1.csv.gz")));
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
        main(Path::new("samples/sample1.csv.gz"))
    }
}
