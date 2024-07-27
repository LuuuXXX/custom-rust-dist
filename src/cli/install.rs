//! Separated module to handle installation related behaviors in command line.

use std::collections::BTreeMap;

use crate::{
    core::manifest::RustToolchain, core::manifest::ToolsetManifest, core::InstallConfiguration,
    core::Installation, rustup::Rustup, utils,
};

use super::{GlobalOpt, Subcommands};

use anyhow::Result;

/// Execute `install` command.
pub(super) fn execute(subcommand: &Subcommands, _opt: GlobalOpt) -> Result<()> {
    let Subcommands::Install {
        prefix,
        registry_url,
        registry_name,
        rustup_dist_server,
        rustup_update_root,
    } = subcommand
    else {
        return Ok(());
    };

    let cargo_registry = registry_url
        .as_ref()
        .map(|u| (registry_name.clone(), u.clone()));

    let config = InstallConfiguration {
        install_dir: prefix
            .clone()
            .unwrap_or_else(utils::home_dir)
            .join(env!("CARGO_PKG_NAME")),
        rustup_dist_server: rustup_dist_server.to_owned(),
        rustup_update_root: rustup_update_root.to_owned(),
        cargo_registry,
    };
    config.init()?;
    config.config_rustup_env_vars()?;
    config.config_cargo()?;

    // TODO: Download manifest form remote server.
    let manifest = ToolsetManifest {
        rust: RustToolchain::new("stable"),
        target: BTreeMap::default(),
        tools: BTreeMap::default(),
    };

    Rustup::init().download_toolchain(&config, &manifest)?;

    // TODO: install third-party tools via cargo that got installed by rustup

    unimplemented!("`install` is not fully yet implemented.")
}
