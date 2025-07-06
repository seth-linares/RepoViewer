//! Self-update handling 
//! 
//! This module provides a wrapper around self_update to ensure that
//! our binary can be updated and replaced safely from GitHub. 

use std::env;

use crate::app_error::AppError;
use self_update::cargo_crate_version;
use self_update::{backends::github::{UpdateBuilder, Update}, Status};

/// Repository configuration constants
const REPO_OWNER: &str = "seth-linares";
const REPO_NAME: &str = "RepoViewer";
const BIN_NAME: &str = "RepoViewer";


/// Show version information for RepoViewer
pub fn show_version_info() {
    println!("\nRepoViewer {}\n", cargo_crate_version!());
    println!("A TUI file explorer for generating directory trees for LLMs\n");
    println!("Repository: https://github.com/seth-linares/RepoViewer\n");
}

/// Check if an update is available without installing it
pub fn check_for_updates() -> Result<(), AppError> {
    println!("\nChecking for updates...");

    let (current_version, latest_version) = get_latest_release_info()?;

    // Compare versions self_update uses semver internally
    if latest_version > current_version {
        println!("Update available!");
        println!("  Current version: v{}", current_version);
        println!("  Latest version:  v{}\n", latest_version);
        println!("Run 'RepoViewer --update' to update to the latest version\n");
    } else {
        println!("You're already on the latest version (v{})\n", current_version);
    }

    Ok(())
}

/// Perform the actual update
pub fn perform_update(skip_confirm: bool) -> Result<(), AppError> {
    println!("\nRepoViewer Self-Updater");
    println!("=======================");

    let (current_version, latest_version) = get_latest_release_info()?;

    // Check if update is needed
    if latest_version <= current_version {
        println!("You're already on the latest version (v{})\n", current_version);
        return Ok(());
    }

    println!("Current version: v{}", current_version);
    println!("Latest version:  v{}\n", latest_version);

    // if we don't skip confirm we need to see if the user really wants to update
    if !skip_confirm {
        println!("This will download and install RepoViewer v{}", latest_version);
        print!("Continue? [Y/n] ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        input = input.trim().to_lowercase();

        if !input.is_empty() && input != "y" && input != "yes" {
            println!("Update cancelled");
            return Err(AppError::UpdateCancelled);
        }
    }

    // now perform the update
    println!("\nDownloading RepoViewer v{}...", latest_version);

    let status = base_updater_config()?
        .show_download_progress(true)
        .no_confirm(true)
        .build()?
        .update()?;

    match status {
        Status::UpToDate(version) => {
            println!("\nAlready up to date (v{})\n", version);
        },
        Status::Updated(version) => {
            println!("\nSuccessfully updated to v{}!", version);
            println!("Please restart RepoViewer to use the new version\n");
        }
    }

    Ok(())

}



/// Create a base updater configuration with common settings so we can adhere to DRY
fn base_updater_config() -> Result<UpdateBuilder, AppError> {
    let target = get_self_update_target();
    
    // Check if platform is supported
    if target == "unsupported" {
        return Err(AppError::UnsupportedPlatform(
            format!("Platform {}-{} is not supported for automatic updates", 
                env::consts::OS, 
                env::consts::ARCH)
        ));
    }

    // We need to return an owned UpdateBuilder so we need to first make a mut var
    // then we can mutate it's state without converting to &mut / losing ownership!!
    let mut builder: UpdateBuilder = Update::configure();

    builder
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(cargo_crate_version!())
        .target(&target);

    Ok(builder)
}


/// Get information about the latest release
fn get_latest_release_info() -> Result<(String, String), AppError> {
    let updater = base_updater_config()?.build()?;

    let latest_release = updater.get_latest_release()?;
    let latest_version = latest_release.version;
    let current_version = cargo_crate_version!();

    Ok((current_version.to_string(), latest_version.to_string()))
}





/// Get the target string that matches our GitHub release binary naming
/// 
/// self_update expects a target triple that matches the binary names in releases
/// Below are the platforms we support for self-updating. So if you compile for another
/// platform and want to use self-update, you'll need to fork this and set it up.
fn get_self_update_target() -> String {
    // Use conditional compilation to determine the target
    // This matches the targets your GitHub Actions builds
    
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    let target = "x86_64-pc-windows-msvc";
    
    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    let target = "x86_64-unknown-linux-gnu";

    #[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "gnu"))]
    let target = "aarch64-unknown-linux-gnu";
    
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let target = "x86_64-apple-darwin";
    
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let target = "aarch64-apple-darwin";
    
    // Fallback for unsupported platforms
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    )))]
    let target = "unsupported";
    
    target.to_string()
}