//! Native resource locations. The Windows layout remains backward-compatible.
use std::path::{Path, PathBuf};
pub fn jackett_name() -> &'static str {
    if cfg!(windows) {
        "JackettConsole.exe"
    } else {
        "jackett"
    }
}
pub fn jackett_asset() -> &'static str {
    if cfg!(windows) {
        "Jackett.Binaries.Windows.zip"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "Jackett.Binaries.macOSARM64.tar.gz"
    } else if cfg!(target_os = "macos") {
        "Jackett.Binaries.macOS.tar.gz"
    } else {
        "Jackett.Binaries.LinuxAMDx64.tar.gz"
    }
}
pub fn amule_relative() -> &'static str {
    if cfg!(windows) {
        "amule/bin/amuled.exe"
    } else if cfg!(target_os = "macos") {
        "amule/aMule.app/Contents/MacOS/amuled"
    } else {
        "amule/bin/amuled"
    }
}
pub fn dev_jackett() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(if cfg!(windows) {
            "../vendor/jackett/Jackett"
        } else {
            "../vendor/native/engine"
        })
        .join(jackett_name())
}
pub fn dev_amule() -> PathBuf {
    if cfg!(windows) {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../vendor/amule-3.1.0/amule-portable-x64/bin/amuled.exe")
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../vendor/native")
            .join(amule_relative())
    }
}
pub fn dev_runtime() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(if cfg!(windows) {
        "../vendor/v2-runtime"
    } else {
        "../vendor/native/v2"
    })
}
pub fn python(runtime: &Path) -> PathBuf {
    runtime.join(if cfg!(windows) {
        "python.exe"
    } else {
        "bin/python3"
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_resources_match_target() {
        assert_eq!(
            python(Path::new("runtime")).file_name().unwrap(),
            if cfg!(windows) {
                "python.exe"
            } else {
                "python3"
            }
        );
        assert!(dev_jackett().ends_with(jackett_name()));
        assert!(dev_amule().ends_with(if cfg!(windows) {
            "amuled.exe"
        } else {
            "amuled"
        }));
    }
}
