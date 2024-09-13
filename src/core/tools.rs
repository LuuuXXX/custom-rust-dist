use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{bail, Result};

use super::{parser::fingerprint::ToolRecord, uninstall::UninstallConfiguration, CARGO_HOME};
use crate::{core::custom_instructions, utils, InstallConfiguration};

#[derive(Debug, PartialEq, PartialOrd)]
/// Representing the structure of an (extracted) tool's directory.
// NB: Mind the order of the variants, they are crucial to installation/uninstallation.
pub(crate) enum Tool<'a> {
    /// Pre-built executable files.
    /// i.e.:
    /// ```text
    /// ├─── some_binary.exe
    /// ├─── cargo-some_binary.exe
    /// ```
    Executables(String, Vec<PathBuf>),
    /// Directory containing `bin` subfolder:
    /// ```text
    /// tool/
    /// ├─── bin/
    /// ├─── ...
    /// ```
    DirWithBin { name: String, path: &'a Path },
    /// We have a custom "script" for how to deal with such directory.
    Custom { name: String, path: &'a Path },
    /// Plugin file, such as `.vsix` files for Visual Studio.
    Plugin {
        name: String,
        kind: PluginType,
        path: &'a Path,
    },
    // `Cargo` just don't make any sense
    #[allow(clippy::enum_variant_names)]
    CargoTool {
        name: String,
        args: Option<Vec<&'a str>>,
    },
}

impl<'a> Tool<'a> {
    pub(crate) fn from_path(name: &str, path: &'a Path) -> Result<Self> {
        if !path.exists() {
            bail!(
                "the path for '{name}' specified as '{}' does not exist.",
                path.display()
            );
        }
        let name = name.to_string();

        // Step 1: Looking for custom instruction
        if custom_instructions::is_supported(&name) {
            return Ok(Self::Custom { name, path });
        }

        // Step 2: Identify from file extension (if it's a file ofc).
        if utils::is_executable(path) {
            return Ok(Self::Executables(name, vec![path.to_path_buf()]));
        } else if path.is_file() {
            let maybe_extension = path.extension();
            if let Some(ext) = maybe_extension.and_then(|ext| ext.to_str()) {
                match ext {
                    "vsix" => {
                        // TODO: When installing, invoke `vscode` plugin install command,
                        // this must be handled after `VS-Code` has been installed,
                        // we might need a `requirements` field in the manifest.
                        return Ok(Self::Plugin {
                            kind: ext.parse()?,
                            path,
                            name,
                        });
                    }
                    _ => bail!("unable to process tool '{name}': unknown file format '{ext}'"),
                }
            }
        }
        // TODO: Well, we got a directory, things are getting complicated, there could be one of this scenarios:
        // 1. Directory contains some executable files and nothing else
        //      Throw these executable files into cargo bin folder
        // 2. Directory contains sub-directory, which look like `bin/ lib/ etc/ ...`
        //      Throw and merge this directories into cargo home. (might be bad, therefore we need a `Manifest.in`!!!)
        // 3. Directory doesn't fit all previous characteristics.
        //      We don't know how to install this tool, throw an error instead.
        else {
            // Step 3: read directory to find characteristics.
            let entries = utils::walk_dir(path, false)?;
            // Check if there is any folder that looks like `bin`
            // Then assuming this is `UsrDirs` type installer.
            if entries.iter().any(|path| path.ends_with("bin")) {
                return Ok(Self::DirWithBin { name, path });
            }
            // If no sub folder exists, and there are binaries lays directly in the folder
            if !entries.iter().any(|path| path.is_dir()) {
                let assumed_binaries = entries
                    .iter()
                    .filter_map(|path| utils::is_executable(path).then_some(path.to_path_buf()));
                return Ok(Self::Executables(name, assumed_binaries.collect()));
            }
        }

        bail!("unable to process tool '{name}' as it is not supported")
    }

    /// Specify as a tool that managed by `cargo`.
    ///
    /// Note: `extra_args` should not contains "install" and `name`.
    pub(crate) fn cargo_tool(name: &str, extra_args: Option<Vec<&'a str>>) -> Self {
        Self::CargoTool {
            name: name.to_string(),
            args: extra_args,
        }
    }

    pub(crate) fn install(&self, config: &InstallConfiguration) -> Result<ToolRecord> {
        match self {
            Self::CargoTool { name, args } => {
                if !config.cargo_is_installed {
                    bail!("trying to install '{name}' using cargo, but cargo is not installed");
                }

                cargo_install_or_uninstall(
                    "install",
                    args.as_deref().unwrap_or(&[name]),
                    config.cargo_home(),
                )?;
                Ok(ToolRecord::cargo_tool())
            }

            Self::Executables(_, exes) => {
                let mut res = vec![];
                for exe in exes {
                    res.push(utils::copy_file_to(exe, config.cargo_bin())?);
                }
                Ok(ToolRecord::with_paths(res))
            }
            Self::Custom { name, path } => {
                let paths = custom_instructions::install(name, path, config)?;
                Ok(ToolRecord::with_paths(paths))
            }
            Self::DirWithBin { name, path } => {
                let tool_dir = install_dir_with_bin_(config, name, path)?;
                Ok(ToolRecord::with_paths(vec![tool_dir]))
            }
            Self::Plugin { kind, path, .. } => {
                // run the installation command.
                kind.install_plugin(path)?;
                // we need to "cache" to installer, so that we could uninstall with it.
                let plugin_backup = utils::copy_file_to(path, config.tools_dir())?;
                Ok(ToolRecord::with_paths(vec![plugin_backup]))
            }
        }
    }

    pub(crate) fn uninstall(&self, config: &UninstallConfiguration) -> Result<()> {
        match self {
            Self::CargoTool { name, args } => {
                cargo_install_or_uninstall(
                    "uninstall",
                    args.as_deref().unwrap_or(&[name]),
                    &config.cargo_home(),
                )?;
            }
            Self::Executables(_, binaries) => {
                for binary in binaries {
                    fs::remove_file(binary)?;
                }
            }
            Self::Custom { name, .. } => custom_instructions::uninstall(name, config)?,
            Self::DirWithBin { path, .. } => uninstall_dir_with_bin_(path)?,
            Self::Plugin { kind, path, .. } => kind.uninstall_plugin(path)?,
        }
        Ok(())
    }
}

fn cargo_install_or_uninstall(op: &str, args: &[&str], cargo_home: &Path) -> Result<()> {
    let mut cargo_bin = cargo_home.to_path_buf();
    cargo_bin.push("bin");
    cargo_bin.push(format!("cargo{}", std::env::consts::EXE_SUFFIX));

    let mut full_args = vec![op];
    full_args.extend_from_slice(args);

    let cargo_home = utils::path_to_str(cargo_home)?;

    utils::execute_with_env(cargo_bin, &full_args, [(CARGO_HOME, cargo_home)])
}

/// Installing [`ToolInstaller::DirWithBin`], with a couple steps:
/// - Move the `tool_dir` to [`tools_dir`](InstallConfiguration::tools_dir).
/// - Add the `bin_dir` to PATH
fn install_dir_with_bin_(
    config: &InstallConfiguration,
    name: &str,
    path: &Path,
) -> Result<PathBuf> {
    let dir = config.tools_dir().join(name);

    utils::move_to(path, &dir, true)?;

    let bin_dir_after_move = dir.join("bin");
    super::os::add_to_path(&bin_dir_after_move)?;
    Ok(dir)
}

/// Uninstalling a tool with bin folder is as simple as removing the directory,
/// and removing the `bin` dir from `PATH`.
fn uninstall_dir_with_bin_(tool_path: &Path) -> Result<()> {
    // Remove from `PATH` at first.
    let bin_dir = tool_path.join("bin");
    super::os::remove_from_path(&bin_dir)?;

    fs::remove_dir_all(tool_path)?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[non_exhaustive]
pub(crate) enum PluginType {
    Vsix,
}

// This list has a fallback order, DO NOT change the order.
pub(crate) static VSCODE_FAMILY: &[&str] =
    &["hwcode", "wecode", "code-exploration", "code-oss", "code"];

impl FromStr for PluginType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "vsix" => Ok(Self::Vsix),
            _ => bail!("unsupprted plugin file type '{s}'"),
        }
    }
}

impl PluginType {
    fn install_or_uninstall_(&self, plugin_path: &Path, uninstall: bool) -> Result<()> {
        match self {
            PluginType::Vsix => {
                for program in VSCODE_FAMILY {
                    if utils::cmd_exist(program) {
                        let op = if uninstall { "uninstall" } else { "install" };
                        let arg_opt = format!("--{op}-extension");
                        println!(
                            "{}",
                            t!(
                                "handling_extension_info",
                                op = op,
                                ext = plugin_path.display(),
                                program = program
                            )
                        );
                        match utils::execute(
                            program,
                            &[arg_opt.as_str(), utils::path_to_str(plugin_path)?],
                        ) {
                            Ok(()) => continue,
                            // Ignore error when uninstalling.
                            Err(_) if uninstall => {
                                println!(
                                    "{}",
                                    t!(
                                        "skip_extension_uninstall_warn",
                                        ext = plugin_path.display(),
                                        program = program
                                    )
                                );
                                continue;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn install_plugin(&self, plugin_path: &Path) -> Result<()> {
        self.install_or_uninstall_(plugin_path, false)
    }

    fn uninstall_plugin(&self, plugin_path: &Path) -> Result<()> {
        self.install_or_uninstall_(plugin_path, true)
    }
}
