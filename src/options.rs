use std::path::PathBuf;
use structopt::StructOpt;
use structopt::clap::arg_enum;

#[derive(Debug, StructOpt)]
#[structopt(about = "A brainfuck compiler.")]
pub struct Options {
    #[structopt(parse(from_os_str))]
    pub source: PathBuf,

    #[structopt(short = "c", long = "cell-size", default_value = "I8", help = "Cell size (I8, I16, I32, I64)")]
    pub cell_size: CellSize,

    #[structopt(short = "m", long = "memory-size", default_value = "4096", help = "The number of cells to allocate")]
    pub memory_size: usize
}

arg_enum! {
    #[derive(Debug)]
    pub enum CellSize {
        I8,
        I16,
        I32,
        I64
    }
}

impl CellSize {
    pub fn bytes(&self) -> usize {
        use CellSize::*;
        match self {
            I8 => 1,
            I16 => 2,
            I32 => 4,
            I64 => 8
        }
    }
}
