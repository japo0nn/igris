#[cfg(target_os = "windows")]
pub static SHELL: &str = "powershell";
#[cfg(target_os = "linux")]
pub static SHELL: &str = "sh";