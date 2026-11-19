use anyhow::Result;
use tracing::info;

pub struct SystemProxy;

impl SystemProxy {
    #[cfg(windows)]
    pub fn enable(port: u16) -> Result<()> {
        use winreg::enums::*;
        use winreg::RegKey;

        info!("Enabling Windows system proxy on 127.0.0.1:{}", port);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (settings, _) = hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings")?;

        let server = format!("127.0.0.1:{}", port);
        settings.set_value("ProxyServer", &server)?;
        settings.set_value("ProxyEnable", &1u32)?;
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn enable(_port: u16) -> Result<()> {
        info!("System proxy automatic config is not supported on this OS yet.");
        Ok(())
    }

    #[cfg(windows)]
    pub fn disable() -> Result<()> {
        use winreg::enums::*;
        use winreg::RegKey;

        info!("Disabling Windows system proxy");
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (settings, _) = hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings")?;
        settings.set_value("ProxyEnable", &0u32)?;
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn disable() -> Result<()> {
        Ok(())
    }
}
