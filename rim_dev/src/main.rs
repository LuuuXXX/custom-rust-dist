#[macro_use]
extern crate rust_i18n;

mod common;
mod dist;
mod mocked;
mod toolkits_parser;
mod vendor;

use anyhow::Result;
use dist::{DistMode, DIST_HELP};
use mocked::{installation, manager, server};
use std::env;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use vendor::{VendorMode, VENDOR_HELP};

i18n!("../locales", fallback = "en");

const HELP: &str = r#"
Usage: cargo dev [OPTIONS] [COMMAND]

Options:
    -h, -help       Print this help message

Commands:
    dist, d         Generate release binaries
    run-manager     Run with manager mode
    vendor          Download packages that are specified in `resource/packages.txt`
    mock-rustup-server
                    Generate a mocked rustup dist server
"#;

const MANAGER_MODE_HELP: &str = r#"
Run with manager mode

Usage: cargo dev run-manager [OPTIONS]

Options:
        --cli       Run manager mode with commandline interface
        --gui       Run manager mode with graphical interface (default)
    -h, -help       Print this help message
"#;
const MOCK_HELP: &str = r#"
Generate a mocked rustup dist server

Usage: cargo dev mock-rustup-server [OPTIONS]

Options:
        --root      Specify another directory for generated files
    -h, -help       Print this help message
"#;

#[derive(Debug)]
enum DevCmd {
    Dist {
        mode: DistMode,
        binary_only: bool,
    },
    RunManager {
        no_gui: bool,
        args: Vec<String>,
    },
    Mock {
        root: Option<PathBuf>,
    },
    Vendor {
        mode: VendorMode,
        name: Option<String>,
        target: Option<String>,
        all_targets: bool,
    },
}

impl DevCmd {
    fn execute(self) -> Result<()> {
        match self {
            Self::Dist { mode, binary_only } => dist::dist(mode, binary_only)?,
            Self::RunManager { no_gui, args } => {
                // a mocked server is needed to run most of function in manager
                server::generate_rim_server_files()?;

                // generate a fake manager binary with higher version so we
                // can test the self update.
                if args.iter().any(|arg| arg == "update") {
                    manager::generate()?;
                }

                installation::generate_and_run_manager(no_gui, &args)?;
            }
            Self::Vendor {
                mode,
                name,
                target,
                all_targets,
            } => vendor::vendor(mode, name, target, all_targets)?,
            Self::Mock { root } => server::generate_rustup_server_files(root)?,
        }
        Ok(())
    }
}

fn current_exe() -> PathBuf {
    env::current_exe().expect("failed to get the path of current binary")
}

fn main() -> Result<ExitCode> {
    let mut args = std::env::args().skip(1);
    let mut stdout = stdout();

    let Some(subcmd) = args.next() else {
        writeln!(&mut stdout, "{HELP}")?;
        return Ok(ExitCode::FAILURE);
    };

    let cmd = match subcmd.to_lowercase().as_str() {
        "-h" | "--help" => {
            writeln!(&mut stdout, "{HELP}")?;
            return Ok(ExitCode::SUCCESS);
        }
        "d" | "dist" => {
            let mut binary_only = false;
            let mut mode = DistMode::Both;

            while let Some(arg) = args.next().as_deref() {
                match arg {
                    "-h" | "--help" => {
                        writeln!(&mut stdout, "{DIST_HELP}")?;
                        return Ok(ExitCode::SUCCESS);
                    }
                    "--cli" => mode = DistMode::Cli,
                    "--gui" => mode = DistMode::Gui,
                    "-b" | "--binary-only" => binary_only = true,
                    _ => (),
                }
            }
            DevCmd::Dist { mode, binary_only }
        }
        "vendor" => {
            let mut name = None;
            let mut mode = VendorMode::Regular;
            let mut target = None;
            let mut all_targets = false;
            while let Some(arg) = args.next().as_deref() {
                match arg {
                    "-h" | "--help" => {
                        writeln!(&mut stdout, "{VENDOR_HELP}")?;
                        return Ok(ExitCode::SUCCESS);
                    }
                    "-a" | "--all-targets" => all_targets = true,
                    "-n" | "--name" => name = args.next(),
                    "--download-only" => mode = VendorMode::DownloadOnly,
                    "--split-only" => mode = VendorMode::SplitOnly,
                    "-t" | "--target" => target = args.next(),
                    s => {
                        writeln!(&mut stdout, "invalid argument '{s}'")?;
                        return Ok(ExitCode::FAILURE);
                    }
                }
            }
            DevCmd::Vendor {
                mode,
                name,
                target,
                all_targets,
            }
        }
        "run-manager" => match args.next().as_deref() {
            Some("-h" | "--help") => {
                writeln!(&mut stdout, "{MANAGER_MODE_HELP}")?;
                return Ok(ExitCode::SUCCESS);
            }
            Some("--cli") => DevCmd::RunManager {
                no_gui: true,
                args: args.collect(),
            },
            _ => DevCmd::RunManager {
                no_gui: false,
                args: args.collect(),
            },
        },
        "mock-rustup-server" => match args.next().as_deref() {
            Some("-r" | "--root") => DevCmd::Mock {
                root: Some(args.next().expect("missing arg value for 'root'").into()),
            },
            Some("-h" | "--help") => {
                writeln!(&mut stdout, "{MOCK_HELP}")?;
                return Ok(ExitCode::SUCCESS);
            }
            _ => DevCmd::Mock { root: None },
        },
        s => {
            writeln!(
                &mut stdout,
                "invalid argument '{s}', check 'cargo dev --help' for available options"
            )?;
            return Ok(ExitCode::FAILURE);
        }
    };
    cmd.execute()?;

    Ok(ExitCode::SUCCESS)
}
