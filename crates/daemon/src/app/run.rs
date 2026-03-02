use tracing::info;
use tracing_subscriber::EnvFilter;

pub fn run() -> anyhow::Result<()> {
    init_tracing();
    info!("kwylock-daemon bootstrap started");
    Ok(())
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("kwylock_daemon=info"));

    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
