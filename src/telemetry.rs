use tracing_subscriber::fmt::format;
use tracing_subscriber::EnvFilter;

pub fn init() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::fmt()
        .event_format(format().pretty())
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
