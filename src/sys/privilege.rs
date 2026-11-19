pub fn is_admin() -> bool {
    #[cfg(windows)]
    {
        // Simple check on Windows: try opening a privileged path or checking TokenElevation
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
        // Check UID == 0 (root)
        unsafe { libc::geteuid() == 0 }
    }
}
