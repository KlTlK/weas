use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::{info, warn};

pub struct SingBoxProcess {
    child: Child,
}

impl SingBoxProcess {
    pub fn spawn(binary_path: &Path, config_path: &Path) -> Result<Self> {
        info!(
            "Spawning sing-box core: {:?} with config: {:?}",
            binary_path, config_path
        );

        let child = Command::new(binary_path)
            .arg("run")
            .arg("-c")
            .arg(config_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn sing-box process")?;

        Ok(Self { child })
    }

    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        let status = self.child.wait().await?;
        Ok(status)
    }

    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping sing-box core process...");
        if let Err(e) = self.child.kill().await {
            warn!("Failed to cleanly kill sing-box process: {:?}", e);
        }
        Ok(())
    }
}

impl Drop for SingBoxProcess {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
