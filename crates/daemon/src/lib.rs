pub mod adapters;
pub mod app;
pub mod domain;

pub fn run() -> anyhow::Result<()> {
    app::run::run()
}
