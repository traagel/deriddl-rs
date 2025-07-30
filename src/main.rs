mod cli;
mod orchestrator;
mod model;
mod tracker;
mod executor;

use clap::Parser;
use cli::args::Cli;
use cli::dispatch::handle;

fn main() {
    env_logger::init();
    let cli = Cli::parse();
    handle(cli);
}
