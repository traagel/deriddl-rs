mod cli;
mod dialects;
mod executor;
mod logger;
mod model;
mod orchestrator;
mod tracker;

use clap::Parser;
use cli::args::Cli;
use cli::dispatch::handle;

fn main() {
    let cli = Cli::parse();
    logger::setup_logger(cli.verbose);
    handle(cli);
}
