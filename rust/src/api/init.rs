use anyhow::Result;
use flutter_rust_bridge::frb;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt::{self, format::FmtSpan}, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter, Layer, Registry};

#[frb]
pub async fn set_lwd(lwd: &str) -> Result<()> {
    let mut state = crate::APPSTATE.lock().await;
    state.set_lwd(lwd);
    Ok(())
}

#[frb]
pub async fn set_db_path(dir: &str, name: &str) -> Result<()> {
    let mut state = crate::APPSTATE.lock().await;
    state.init_db(dir, name).await?;
    state.initialized = true;
    Ok(())
}

#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
    let _ = env_logger::builder().try_init();
    let _ = Registry::default()
        .with(default_layer())
        .with(env_layer())
        .try_init();
    let _ = rustls::crypto::ring::default_provider().install_default();
}

type BoxedLayer<S> = Box<dyn Layer<S> + Send + Sync + 'static>;

fn default_layer<S>() -> BoxedLayer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fmt::layer()
        .with_ansi(false)
        .with_span_events(FmtSpan::ACTIVE)
        .compact()
        .boxed()
}

fn env_layer<S>() -> BoxedLayer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        .boxed()
}
