use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt::{
        self,
        format::FmtSpan,
    },
    layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
    EnvFilter, Layer, Registry,
};

pub fn init_logger() {
    // Default utilities - feel free to customize
    // flutter_rust_bridge::setup_default_user_utils();
    let _ = Registry::default()
        .with(default_layer())
        .with(env_layer())
        .try_init();
    tracing::info!("Rust logging initialized");
}

pub type BoxedLayer<S> = Box<dyn Layer<S> + Send + Sync + 'static>;

pub fn default_layer<S>() -> BoxedLayer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fmt::layer()
        .with_ansi(false)
        .with_span_events(FmtSpan::ACTIVE)
        .compact()
        .boxed()
}

pub fn env_layer<S>() -> BoxedLayer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        .boxed()
}
