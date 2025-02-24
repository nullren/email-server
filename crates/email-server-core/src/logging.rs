use once_cell::sync::Lazy;
use tracing_subscriber::EnvFilter;

static INIT: Lazy<()> = Lazy::new(|| {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_writer(std::io::stderr)
        .init();
});

pub fn setup() {
    Lazy::force(&INIT); // Ensure logging is initialized once
}
