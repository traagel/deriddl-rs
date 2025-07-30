use env_logger::{Builder, Target};
use log::Level;
use std::env;
use std::io::Write;

pub fn setup_logger(verbose: bool) {
    let level = if verbose { Level::Debug } else { Level::Info };

    let mut builder = Builder::new();
    builder.filter(None, level.to_level_filter());
    builder.target(Target::Stdout);

    builder.format(|buf, record| {
        let emoji = match record.level() {
            Level::Error => "❌ ",
            Level::Warn => "⚠️  ",
            Level::Info => "",
            Level::Debug => "",
            Level::Trace => "",
        };
        writeln!(buf, "{}{}", emoji, record.args())
    });

    if env::var("RUST_LOG").is_ok() {
        builder.parse_default_env();
    }

    builder.init();
}
