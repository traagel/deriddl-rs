use log::{info, debug};

pub fn run_plan(conn: &str, path: &str) {
    info!("Generating migration plan");
    debug!("Connection: {}", conn);
    debug!("Migrations path: {}", path);

    // TODO: Print list of unapplied migrations (dry-run mode)
}
