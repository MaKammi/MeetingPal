use anyhow::{ensure, Context, Result};
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri_plugin_updater::UpdaterExt;

fn client(version: &str) -> Result<tauri::App<MockRuntime>> {
    let config: tauri::Config = serde_json::from_str(include_str!("../tauri.conf.json"))?;
    let mut context = mock_context(noop_assets());
    context.config_mut().plugins = config.plugins;
    context.package_info_mut().version = version.parse()?;
    Ok(mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)?)
}

/// Uses the real updater and public release assets, without running an installer.
#[tokio::test]
#[ignore = "requires the current version to be published, downloads its update package"]
async fn public_feed_supplies_and_verifies_the_platform_update() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let latest = client(version)?;
    ensure!(
        latest.updater()?.check().await?.is_none(),
        "unexpected update for current version"
    );
    let older = client("0.0.0")?;
    let update = older
        .updater()?
        .check()
        .await?
        .context("missing update for older version")?;
    ensure!(update.version == version, "feed does not match the release");
    let bytes = update.download(|_, _| {}, || {}).await?;
    ensure!(bytes.len() > 1_000_000, "unexpectedly small package");
    println!(
        "Verified {} bytes from {} for {version}",
        bytes.len(),
        update.download_url
    );
    Ok(())
}
