pub fn is_admin() -> bool {
    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("net")
            .arg("session")
            .output();
        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(unix)]
    {
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
            .unwrap_or(false)
    }

    #[cfg(not(any(windows, unix)))]
    {
        false
    }
}
