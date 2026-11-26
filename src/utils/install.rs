// Issue #40: standalone install durability self-check.
// The FusionMLX desktop app writes a CLI shim that execs the binary out of
// the app bundle; when the app runs from a mounted DMG, the shim (and any
// binary reached through it) points into a read-only volume that dies on
// eject. Classify the running executable's path so `fusion version` and
// `fusion doctor` can flag ephemeral installs.

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InstallKind {
    // Durable standalone install (~/.fusion/bin, /usr/local, /opt, home...).
    Durable,
    // DMG mount, .app bundle interior, or temp dir — dies on eject/reboot.
    Ephemeral,
}

// Classify the running executable's path. Best-effort: current_exe()
// failures degrade to Ephemeral (fail visible — an unknown path is
// treated as not trustworthy rather than silently fine).
pub fn classify_current_exe() -> (InstallKind, String) {
    let path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    (classify_path(&path), path)
}

pub fn classify_path(path: &str) -> InstallKind {
    if path.is_empty() {
        return InstallKind::Ephemeral;
    }
    let p = path;
    // /Volumes/<dmg-mount>/... and .app bundle interiors are ephemeral.
    if p.starts_with("/Volumes/") || p.contains(".app/Contents/") {
        return InstallKind::Ephemeral;
    }
    // Temp dirs never survive a reboot and are not a distribution target.
    if p.starts_with("/tmp/")
        || p.starts_with("/private/tmp/")
        || p.contains("/target/debug/")
        || p.contains("/target/release/")
    {
        return InstallKind::Ephemeral;
    }
    // Known durable prefixes: the standalone install dir, system bins,
    // opt, and any user-home path. Prefix matching on the canonical home
    // layout (/Users, /home) keeps this deterministic — env-dependent
    // home_dir() lookups are unreliable in test sandboxes.
    if p.starts_with("/usr/local/") || p.starts_with("/opt/") || p.starts_with("/usr/bin/") {
        return InstallKind::Durable;
    }
    if p.starts_with("/Users/") || p.starts_with("/home/") {
        return InstallKind::Durable;
    }
    InstallKind::Ephemeral
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmg_and_bundle_paths_are_ephemeral() {
        assert_eq!(
            classify_path("/Volumes/FusionMLX-1.0/FusionMLX.app/Contents/MacOS/fusion"),
            InstallKind::Ephemeral
        );
        assert_eq!(
            classify_path("/Applications/FusionMLX.app/Contents/MacOS/bin/fusion"),
            InstallKind::Ephemeral
        );
    }

    #[test]
    fn temp_and_cargo_target_paths_are_ephemeral() {
        assert_eq!(classify_path("/tmp/x/fusion"), InstallKind::Ephemeral);
        assert_eq!(
            classify_path("/private/tmp/x/fusion"),
            InstallKind::Ephemeral
        );
        assert_eq!(
            classify_path("/Users/d/fusion-cli/target/release/fusion"),
            InstallKind::Ephemeral
        );
    }

    #[test]
    fn empty_path_is_ephemeral_not_silent_ok() {
        assert_eq!(classify_path(""), InstallKind::Ephemeral);
    }

    #[test]
    fn durable_prefixes_are_durable() {
        assert_eq!(
            classify_path("/Users/dahai/.fusion/bin/fusion"),
            InstallKind::Durable
        );
        assert_eq!(classify_path("/usr/local/bin/fusion"), InstallKind::Durable);
        assert_eq!(
            classify_path("/opt/homebrew/bin/fusion"),
            InstallKind::Durable
        );
    }

    #[test]
    fn app_shim_through_public_bin_is_durable_when_target_durable() {
        // The public bin dir lives under the user home even though it may
        // symlink to the app; path-level classification sees the symlink
        // target only via current_exe() resolution — a resolved durable
        // target stays durable.
        assert_eq!(
            classify_path("/Users/dahai/.fusion-mlx/bin/fusion"),
            InstallKind::Durable
        );
    }
}
