#![warn(clippy::unwrap_used)]
#![warn(clippy::panic)]
use csv::StringRecord;
use flate2::read::GzDecoder;
use log::*;
use std::f32;
use std::io::prelude::*;
use std::path::Path;
use std::{cmp::Ordering, ffi::OsStr, fs::File};
mod palettes;
use anyhow::{Context, Result};
use arrayvec::ArrayVec;
use image::png::PngEncoder;
use itertools::Itertools;
pub use palettes::{scale_tocolor, Palette};

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
    fn new(record: StringRecord) -> Result<Measurement> {
        let mut values: Vec<_> = record
            .iter()
            .skip(6)
            .map(|s| parse_f32(s))
            .collect::<Result<Vec<_>>>()?;
        if values.len() > 1 {
            values.remove(values.len() - 1);
        }
        Ok(Measurement {
            date: record
                .get(0)
                .context("Couldn't get date column")?
                .to_string(),
            time: record
                .get(1)
                .context("Couldn't get time column")?
                .to_string(),
            freq_low: record
                .get(2)
                .context("Couldn't get freq_low column")?
                .parse()?,
            freq_high: record
                .get(3)
                .context("Couldn't get freq_high column")?
                .parse()?,
            freq_step: record
                .get(4)
                .context("Couldn't get freq_step column")?
                .parse()?,
            samples: record
                .get(5)
                .context("Couldn't get samples column")?
                .parse()?,
            values,
        })
    }
}

#[derive(PartialEq, Debug)]
pub struct Summary {
    pub min: f32,
    pub max: f32,
    pub width: usize,
}

impl Summary {
    fn empty() -> Self {
        Self {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
            width: 0,
        }
    }

    fn update(a: Self, val: f32, width: usize) -> Self {
        Self {
            min: {
                let a = a.min;
                let b = val;
                if a.is_finite() {
                    a.min(b)
                } else {
                    b
                }
            },
            max: {
                let a = a.max;
                let b = val;
                if a.is_finite() {
                    a.max(b)
                } else {
                    b
                }
            },
            width,
        }
    }
}

fn parse_f32(s: &str) -> Result<f32> {
    if s == "-nan" || s == "nan" {
        Ok(f32::NAN)
    } else {
        Ok(s.parse::<f32>()?)
    }
}

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Box<dyn std::io::Read>> {
    let path = path.as_ref();
    let file = File::open(path).context(format!("Couldn't open file '{}'", path.display()))?;
    match path.extension() {
        Some(ext) if ext == OsStr::new("gz") => Ok(Box::new(GzDecoder::new(file))),
        _ => Ok(Box::new(file)),
    }
}

fn read_file<T: std::io::Read>(file: T) -> csv::Reader<T> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file)
}

pub fn main<P: AsRef<Path>>(path: P, palette: Palette) -> Result<()> {
    let path = path.as_ref();
    info!("Loading: {}", path.display());
    //Preprocess
    let file = open_file(path)?;
    let summary = preprocess_iter(file)?;
    info!("Color values {} to {}", summary.min, summary.max);
    //Process
    let file = open_file(path).context("Couldn't preprocess file")?;
    let reader = read_file(file);
    let (datawidth, dataheight, img) =
        process(reader, summary.min, summary.max, palette).context("Couldn't process file")?;
    //Draw
    let (height, imgdata) = create_image(datawidth, dataheight, img);
    let dest = path.with_extension("png");
    save_image(datawidth, height, imgdata, dest)?;
    Ok(())
}

pub fn preprocess(file: Box<dyn Read>) -> Result<Summary> {
    let reader = read_file(file);
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut width: Option<usize> = None;
    let mut first_date = None;
    for result in reader.into_records() {
        let record = result.map(|mut x| {
            x.trim();
            x
        })?;

        let timestamp = record
            .get(0)
            .and_then(|date| record.get(1).map(|time| format!("{} {}", date, time)));

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

        let values_count = values.len() - 1;
        if first_date == None {
            first_date = timestamp;
            width = Some(values_count);
        } else if first_date == timestamp {
            width = width.map(|v| v + values_count);
        }

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
    Ok(Summary {
        min,
        max,
        width: width.ok_or_else(|| {
            anyhow::anyhow!("width sohuld be defined if there's at least one row of data")
        })?,
    })
}

pub fn preprocess_iter(file: Box<dyn Read>) -> Result<Summary> {
    fn trim(mut record: StringRecord) -> StringRecord {
        record.trim();
        record
    }

    fn get_datetime_if_not_err(res: &csv::Result<StringRecord>) -> String {
        match res {
            Ok(sr) => {
                format!(
                    "{} {}",
                    sr.get(0).unwrap_or("empty").to_string(),
                    sr.get(1).unwrap_or("empty").to_string()
                )
            }
            Err(_) => "err".to_string(),
        }
    }

    fn parse_f32(s: &str) -> Result<f32> {
        if s == "-nan" || s == "nan" {
            Ok(f32::NAN)
        } else {
            s.parse::<f32>()
                .with_context(|| anyhow::anyhow!("'{}' should be a valid float", s))
        }
    }

    fn get_values(record: StringRecord) -> Result<impl Iterator<Item = f32>> {
        let mut vals = record
            .into_iter()
            .skip(6)
            .map(parse_f32)
            .collect::<Result<Vec<f32>>>()?;
        vals.pop()
                    .ok_or_else(||anyhow::anyhow!("there should be at least one value in a row, so we should be able to skip the last one"))?;
        Ok(vals.into_iter())
    }

    fn map_group(
        (_, group): (String, impl Iterator<Item = csv::Result<StringRecord>>),
    ) -> Result<impl Iterator<Item = f32>> {
        Ok(group
            .map(|record| {
                record
                    .context("failed to parse record")
                    .and_then(get_values)
                    .map(|x| x.collect_vec())
            })
            .collect::<Result<Vec<Vec<f32>>>>()?
            .into_iter()
            .flat_map(|x| x.into_iter()))
    }

    fn fold_vals(summary: Result<Summary>, vals: impl Iterator<Item = f32>) -> Result<Summary> {
        let vals = vals.collect_vec();
        summary.map(|sum| {
            let width = vals.len();
            vals.into_iter()
                .fold(sum, |sum, val| Summary::update(sum, val, width))
        })
    }

    read_file(file)
        .into_records()
        .map_ok(trim)
        .group_by(get_datetime_if_not_err)
        .into_iter()
        .map(map_group)
        .fold_ok(Ok(Summary::empty()), fold_vals)?
}

pub fn process<R: Read>(
    reader: csv::Reader<R>,
    min: f32,
    max: f32,
    palette: Palette,
) -> Result<(usize, usize, std::vec::Vec<u8>)> {
    let mut date: String = "".to_string();
    let mut time: String = "".to_string();
    let mut batch = 0;
    let mut datawidth = 0;
    let mut img = Vec::new();
    for result in reader.into_records() {
        let mut record = result?;
        record.trim();
        assert!(record.len() >= 7);
        let m = Measurement::new(record)?;
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
            let pixel = scale_tocolor(palette, v, min, max);
            img.extend(pixel.iter());
            batch += 1;
        }
    }
    if datawidth == 0 {
        datawidth = batch;
    }
    let w = datawidth;
    let h = img.len() / 3 / datawidth;
    info!("Img data {}x{}", w, h);
    Ok((w, h, img))
}

pub fn process_iter<R: Read>(
    reader: csv::Reader<R>,
    min: f32,
    max: f32,
    width: usize,
) -> Result<(usize, usize, std::vec::Vec<u8>)> {
    let img: Vec<u8> = reader
        .into_records()
        .map(|res| -> anyhow::Result<Measurement> {
            let mut record = res.context("Invalid CSV record")?;
            debug_assert!(record.len() >= 7);
            record.trim();
            Measurement::new(record)
        })
        .map(|m| match m {
            Ok(m) => Ok(m.values.into_iter().flat_map(|val| {
                ArrayVec::from(scale_tocolor(Palette::Default, val, min, max)).into_iter()
            })),
            Err(e) => Err(e),
        })
        .flat_map(|r| match r {
            Ok(inner) => inner,
            Err(e) => {
                panic!("{:?}", e)
            }
        })
        .collect();

    Ok((width, img.len() / 3 / width, img))
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

fn save_image<P: std::convert::AsRef<std::path::Path>>(
    width: usize,
    height: usize,
    imgdata: Vec<u8>,
    dest: P,
) -> Result<()> {
    info!("Saving {} {}x{}", dest.as_ref().display(), width, height);
    let f = std::fs::File::create(dest)?;
    PngEncoder::new(f).encode(
        &imgdata,
        width as u32,
        height as u32,
        image::ColorType::Rgb8,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_generator::test_resources;
    use webp::PixelLayout;

    #[test]
    fn preprocess_basic_result() -> Result<()> {
        let res = preprocess(open_file(Path::new("samples/46M.csv.gz"))?)?;
        assert_eq!(
            res,
            Summary {
                min: -29.4,
                max: 21.35,
                width: 11622,
            }
        );
        Ok(())
    }

    #[test]
    fn webp_new_image() {
        let size =
            (*webp::Encoder::new(&[0, 0, 0], PixelLayout::Rgb, 1, 1).encode_lossless()).len();
        assert_ne!(0, size);
    }

    #[test]
    fn preprocess_iter_result() -> Result<()> {
        let res = preprocess_iter(open_file(Path::new("samples/46M.csv.gz"))?)?;
        assert_eq!(
            res,
            Summary {
                min: -29.4,
                max: 21.35,
                width: 11622,
            }
        );
        Ok(())
    }

    #[test_resources("samples/*.csv.gz")]
    fn process_implementations_equal(path: &str) -> Result<()> {
        let sum = preprocess_iter(open_file(path)?)?;
        let basic = process(
            read_file(open_file(path)?),
            sum.min,
            sum.max,
            Palette::Default,
        )?;
        let iter = process_iter(read_file(open_file(path)?), sum.min, sum.max, sum.width)?;

        assert!(basic.2 == iter.2, "Results differ");
        assert_eq!(basic.0, iter.0, "Widths differ");
        assert_eq!(basic.1, iter.1, "Heights differ");
        Ok(())
    }

    #[test_resources("samples/*.csv.gz")]
    fn complete_gzip(path: &str) -> Result<()> {
        main(path, Palette::Default)
    }

    #[test_resources("samples/*.csv")]
    fn complete_plain(path: &str) -> Result<()> {
        main(path, Palette::Default)
    }
}
