use csv::StringRecord;
use csv::StringRecordsIntoIter;
use image::png::PNGEncoder;
use log::info;

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

fn normalize(v: f32, min: f32, max: f32) -> Vec<u8> {
    let n = (v - min) / max * 256.0;
    vec![n as u8, n as u8, 50]
}

fn read_file(path: &str) -> StringRecordsIntoIter<std::fs::File> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)
        .unwrap()
        .into_records()
}

pub fn main(path: &str) {
    info!("Loading: {}", path);
    let records = read_file(path);
    let (datawidth, dataheight, img) = process(records);
    let (height, imgdata) = create_image(datawidth, dataheight, img);
    let dest = path.to_owned() + ".png";
    save_image(datawidth, height, imgdata, &dest).unwrap();
}

fn process(records: StringRecordsIntoIter<std::fs::File>) -> (usize, usize, std::vec::Vec<u8>) {
    let mut date: String = "".to_string();
    let mut time: String = "".to_string();
    let mut batch = Vec::new();
    let mut datawidth = 0;
    let mut img = Vec::new();
    for result in records {
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
        img.extend(vals.iter().flat_map(|(_, v)| normalize(*v, -17.0, 20.0)));
        batch.extend(vals);
    }
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
    (height + 26, imgdata)
}

fn save_image(
    width: usize,
    height: usize,
    imgdata: Vec<u8>,
    dest: &str,
) -> std::result::Result<(), image::error::ImageError> {
    info!("Saving target/1.png {}x{}", width, height);
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

    #[test]
    fn freq() {}
}
