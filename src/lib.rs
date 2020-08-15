use csv::StringRecord;
use flate2::read::GzDecoder;
use image::png::PNGEncoder;
use log::*;
use std::f32;
use std::io::prelude::*;
use std::path::Path;
use std::{cmp::Ordering, ffi::OsStr, fs::File};
mod palettes;
use arrayvec::ArrayVec;
use palettes::default::DefaultPalette;
use palettes::scale_tocolor;

#[derive(Debug)]
struct Measurement {
    date: String,
    time: String,
    freq_low: u64,
    freq_high: u64,
    freq_step: f64,
    samples: u32,
    values: Vec<f32>,
}

impl Measurement {
    fn get_values_with_freq(&self) -> Vec<(f64, f32)> {
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
        let mut values: Vec<_> = record
            .iter()
            .skip(6)
            .map(|s| parse_f32(s).unwrap())
            .collect();
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
    debug_assert!(parsed.is_ok(), "Could not parse '{}'", string);
    parsed
}

fn parse_f32(s: &str) -> std::result::Result<f32, <f32 as std::str::FromStr>::Err> {
    if s == "-nan" || s == "nan" {
        Ok(f32::NAN)
    } else {
        parse(s)
    }
}

pub fn open_file<P: AsRef<Path>>(path: P) -> Box<dyn std::io::Read> {
    let path = path.as_ref();
    let file = File::open(path).unwrap();
    match path.extension() {
        Some(ext) if ext == OsStr::new("gz") => Box::new(GzDecoder::new(file)),
        _ => Box::new(file),
    }
}

fn read_file<T: std::io::Read>(file: T) -> csv::Reader<T> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file)
}

pub fn main<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    info!("Loading: {}", path.display());
    //Preprocess
    let file = open_file(path);
    let summary = preprocess_iter(file);
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
                if s == "-nan" || s == "nan" {
                    f32::NAN
                } else {
                    s.trim()
                        .parse::<f32>()
                        .unwrap_or_else(|e| panic!("'{}' should be a valid float: {:?}", s, e))
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
        .map(|x| {
            let mut x = x.unwrap();
            x.trim();
            x
        })
        .flat_map(|line| {
            line.into_iter()
                .skip(6)
                .map(|s| {
                    if s == "-nan" || s == "nan" {
                        f32::NAN
                    } else {
                        s.parse::<f32>().unwrap_or_else(|e| {
                            panic!("'{}' should be a valid float: '{:?}'", s, e)
                        })
                    }
                })
                .collect::<Vec<f32>>()
        })
        .fold(Summary::empty(), Summary::combine_f32)
}

pub fn process<R: Read>(
    reader: csv::Reader<R>,
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
        let vals = m.get_values_with_freq();
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
            let pixel = scale_tocolor(Box::from(DefaultPalette {}), v, min, max);
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

pub fn process_iter<R: Read>(
    reader: csv::Reader<R>,
    min: f32,
    max: f32,
) -> (usize, usize, std::vec::Vec<u8>) {
    let img: Vec<u8> = reader
        .into_records()
        .map(|res| {
            let mut record = res.expect("Invalid CSV record");
            debug_assert!(record.len() > 7);
            record.trim();
            record
        })
        .map(Measurement::new)
        .flat_map(|m| m.values.into_iter())
        .flat_map(|val| {
            let slice = scale_tocolor(Box::from(DefaultPalette {}), val, min, max);
            ArrayVec::from(slice).into_iter()
        })
        .collect();

    (1, img.len() / 3 / 1, img)
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
    use test_generator::test_resources;
    use webp::PixelLayout;

    #[test]
    fn preprocess_basic_result() {
        let res = preprocess(open_file(Path::new("samples/46M.csv.gz")));
        assert_eq!(
            res,
            Summary {
                min: -29.4,
                max: 21.35
            }
        );
    }

    #[test]
    fn webp_new_image() {
        let size =
            (*webp::Encoder::new(&[0, 0, 0], PixelLayout::Rgb, 1, 1).encode_lossless()).len();
        assert_ne!(0, size);
    }

    #[test]
    fn preprocess_iter_result() {
        let res = preprocess_iter(open_file(Path::new("samples/46M.csv.gz")));
        assert_eq!(
            res,
            Summary {
                min: -29.4,
                max: 21.35
            }
        );
    }

    #[test_resources("samples/*.csv.gz")]
    fn process_implementations_equal(path: &str) {
        let basic = process(read_file(open_file(path)), -1000.0, 1000.0);
        let iter = process_iter(read_file(open_file(path)), -1000.0, 1000.0);
        assert_eq!(basic, iter)
    }

    #[test_resources("samples/*.csv.gz")]
    fn complete(path: &str) {
        main(path)
    }
}
