#![warn(clippy::unwrap_used)]
#![warn(clippy::panic)]
use anyhow::Result;
use anyhow::{anyhow, Context};
use log::{debug, warn};
use sdr_heatmap::Palette;
use std::{path::PathBuf, str::FromStr};
use walkdir::WalkDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum OptPalette {
    Default,
    Extended,
}

impl FromStr for OptPalette {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(OptPalette::Default),
            "extended" => Ok(OptPalette::Extended),
            _ => Err(anyhow!("{} is not a valid palette name", s)),
        }
    }
}
impl From<OptPalette> for Palette {
    fn from(opt: OptPalette) -> Self {
        match opt {
            OptPalette::Default => Palette::Default,
            OptPalette::Extended => Palette::Extended,
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = NAME, about = "Render .csv from rtl_power into images. Based on heatmap.py", version = VERSION, author = AUTHOR)]
struct Opt {
    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,

    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,

    /// Finds .csv files in the specified folder and runs on all of them
    #[structopt(short = "r", long = "recursive")]
    recursive: bool,

    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Choose a function that converts signal value to a color. (Default: RGB: [0-255,0-255,50], Extended: like default, with more steps)
    #[structopt(short, long, default_value = "default")]
    palette: OptPalette,
}

fn main() -> Result<()> {
    let options: Opt = Opt::from_args();

    stderrlog::new()
        .module(module_path!())
        .quiet(options.quiet)
        .verbosity(options.verbose)
        .init()?;

    debug!("Options: {:?}", options);

    let input = options.input;
    let exts = vec![".csv", ".csv.gz"];
    let palette = options.palette.into();

    if options.recursive {
        for entry in WalkDir::new(input) {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy();
            if exts.iter().any(|ext| name.ends_with(ext)) {
                sdr_heatmap::main(entry.path(), palette)
                    .context(format!("Error on file '{}'", entry.path().display()))?;
            }
        }
    } else {
        sdr_heatmap::main(&input, palette)
            .context(format!("Error on file '{}'", input.display()))?;
    };
    Ok(())
}
#[cfg(test)]
mod tests {}
