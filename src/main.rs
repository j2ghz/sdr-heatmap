use clap::{App, Arg};
use walkdir::WalkDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

fn main() {
    let matches = App::new(NAME)
        .version(VERSION)
        .author(AUTHOR)
        .about("Render .csv from rtl_power into images. Based on heatmap.py")
        .arg(
            Arg::with_name("INPUT")
                .help("Specify the .csv file to use")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .help("Silence all output"),
        )
        .arg(
            Arg::with_name("recursive")
                .short("r")
                .multiple(true)
                .help("Finds .csv files in the specified folder and runs on all fo them"),
        )
        .get_matches();

    let verbose = matches.occurrences_of("verbose") as usize;
    let quiet = matches.is_present("quiet");
    stderrlog::new()
        .module(module_path!())
        .quiet(quiet)
        .verbosity(verbose)
        //.timestamp(ts)
        .init()
        .unwrap();

    let input = matches.value_of("INPUT").unwrap();
    let exts = vec![".csv", ".csv.gz"];

    if matches.is_present("recursive") {
        for entry in WalkDir::new(input) {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy();
            if exts.iter().any(|ext| name.ends_with(ext)) {
                sdr_heatmap::main(entry.path().to_str().unwrap());
            }
        }
    } else {
        sdr_heatmap::main(input);
    }
}
