use clap::{App, Arg};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

fn main() {
    let matches = App::new(NAME)
        .version(VERSION)
        .author(AUTHOR)
        .about("Render .csv from rtl_power into images. Based on heatmap.py")
        .arg(
            Arg::with_name("CSV")
                .help("Specify the .csv file to use")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .get_matches();
    let input = matches.value_of("CSV").unwrap();
    sdr_heatmap::main(input);
}
