use crate::common;

use super::TOOLKIT_NAME;
use anyhow::{anyhow, Context, Result};
use sha2::Digest;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// The version list of rust toolchain
static VERSIONS: &[&str] = &["1.80.1", "1.81.0", "1.82.0", "1.84.1"];
/// The date of each rust toolchain to be destributed,
/// make sure the length matches [`VERSIONS`].
static DATES: &[&str] = &["2024-08-08", "2024-09-05", "2024-10-17", "2025-01-30"];
// TARGETS and COMPONENTS are needed to generate mocked component packages
static TARGETS: &[&str] = &[
    "aarch64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    "x86_64-pc-windows-gnu",
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
];
static COMPONENTS: &[Component] = &[
    Component::new("rustc"),
    Component::new("cargo"),
    Component::new_with_multi_target("rust-std", TARGETS),
    Component::new_with_multi_target("rust-mingw", &["x86_64-pc-windows-gnu"]),
    Component::new("rust-docs"),
    Component::new("rustfmt"),
    Component::new("clippy"),
    Component::new("rust-analyzer"),
    Component::new_with_no_target("rust-src"),
    Component::new("llvm-tools"),
    Component::new("rustc-dev"),
];

static RENAMES: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| {
    HashMap::from_iter([
        ("clippy", "clippy-preview"),
        ("rustfmt", "rustfmt-preview"),
        ("llvm-tools", "llvm-tools-preview"),
        ("rust-analyzer", "rust-analyzer-preview"),
    ])
});

struct Component {
    name: &'static str,
    target: ComponentTarget<'static>,
}

impl Component {
    /// Return a component with given name under current target.
    const fn new(name: &'static str) -> Self {
        Self {
            name,
            target: ComponentTarget::Single(env!("TARGET")),
        }
    }
    const fn new_with_multi_target(name: &'static str, targets: &'static [&'static str]) -> Self {
        Self {
            name,
            target: ComponentTarget::Multi(targets),
        }
    }
    const fn new_with_no_target(name: &'static str) -> Self {
        Self {
            name,
            target: ComponentTarget::NoTarget,
        }
    }
}

enum ComponentTarget<'a> {
    NoTarget,
    Single(&'a str),
    Multi(&'a [&'a str]),
}

struct RimServer {
    dist_dir: PathBuf,
}

impl RimServer {
    fn new() -> Self {
        let dist_dir = super::rim_server_dir().join("dist");
        fs::create_dir_all(&dist_dir)
            .unwrap_or_else(|_| panic!("unable to create mocked dist dir"));
        Self { dist_dir }
    }

    fn gen_dist_manifest(&self) -> Result<()> {
        let server_url = common::path_to_url(super::rim_server_dir());
        let dist_manifest_content_for = |ver: &str| -> String {
            format!(
                "
[[packages]]
name = \"{TOOLKIT_NAME}\"
version = \"stable-{ver}\"
desc = \"This is is generated for testing purpose\"
info = '''
- A fake toolkit with fake info that is generated by rim-dev
'''
manifest-url = \"{}/dist/stable-{ver}.toml\"
",
                server_url.as_str()
            )
        };

        let mut full_content = String::new();
        for ver in VERSIONS {
            full_content.push_str(&dist_manifest_content_for(ver));
        }

        let dist_manifest = self.dist_dir.join("distribution-manifest.toml");
        fs::write(dist_manifest, full_content).context("unable to create dist-manifest")
    }

    fn gen_toolset_manifests(&self) -> Result<()> {
        let toolset_manifest_for = |ver: &str| -> String {
            format!(
                "
name = \"{TOOLKIT_NAME}\"
version = \"stable-{ver}\"

[rust]
version = \"{ver}\"
group = \"Rust\"
components = [\"clippy\", \"rustfmt\", \"rust-src\", \"rust-docs\"]
optional-components = [\"llvm-tools\", \"rustc-dev\", \"rust-analyzer\"]

[rust.profile]
name = \"minimal\"
verbose-name = \"Basic\"
description = \"Basic set of tools to use Rust properly\"

[tools.descriptions]
llvm-tools = \"llvm-tools\"
rustc-dev = \"rustc-dev\"
rust-analyzer = \"rust-analyzer\"
",
            )
        };

        for ver in VERSIONS {
            let content = toolset_manifest_for(ver);
            // the name should match the ones in `gen_dist_manifest`
            let filename = format!("stable-{ver}.toml");
            let dest = self.dist_dir.join(filename);
            fs::write(dest, content)?;
        }

        Ok(())
    }
}

/// Generated rustup dist server, for test purpose only
struct RustupServer {
    dist_dir: PathBuf,
}

impl RustupServer {
    fn new(root: Option<PathBuf>) -> Self {
        let dist_dir = root.unwrap_or_else(super::rustup_server_dir).join("dist");
        fs::create_dir_all(&dist_dir)
            .unwrap_or_else(|_| panic!("unable to create mocked dist dir"));

        Self { dist_dir }
    }

    /// Generate the dist directory, where all toolchain components stored.
    ///
    /// Toolchain dist directory contains two major parts:
    /// 1. Package tarballs (with checksum) in date folders
    /// 2. Channel manifests (with checksum)
    fn gen_dist_dir(&self) -> Result<()> {
        for (date, version) in DATES.iter().zip(VERSIONS) {
            println!("generating toolchain with date: {date}, version: {version}");
            let mut checksum_pair = vec![];
            self.gen_date_folder_(date, version, &mut checksum_pair)?;
            self.gen_channel_manifest_(date, version, &checksum_pair)?;
        }
        Ok(())
    }

    fn gen_channel_manifest_(
        &self,
        date: &str,
        version: &str,
        checksums: &[(String, String)],
    ) -> Result<()> {
        if checksums.is_empty() {
            anyhow::bail!(
                "missing checksum for components, \
                make sure to call `gen_date_folder_` before calling this"
            );
        }

        let template_path = common::resources_dir()
            .join("templates")
            .join("channel-rust");
        if !template_path.is_file() {
            anyhow::bail!(
                "unable to find template file under resource directory \n\
                make sure to run `./resources/unpack_template.sh` script to unpack it"
            );
        }

        let output_path = self.dist_dir.join(format!("channel-rust-{version}.toml"));
        let new_content =
            self.modified_channel_manifest_(&template_path, date, version, checksums)?;

        fs::write(&output_path, new_content)?;
        write_checksum(&output_path)?;
        Ok(())
    }

    fn gen_date_folder_(
        &self,
        date: &str,
        version: &str,
        checksums: &mut Vec<(String, String)>,
    ) -> Result<()> {
        let date_dir = self.dist_dir.join(date);
        common::ensure_dir(&date_dir)?;

        for component in COMPONENTS {
            let targets = match component.target {
                ComponentTarget::NoTarget => {
                    // generate one file then continue
                    self.gen_component_pkg_(&date_dir, component.name, version, None, checksums)?;
                    continue;
                }
                ComponentTarget::Single(target) => vec![target],
                ComponentTarget::Multi(targets) => targets.to_vec(),
            };
            for target in targets {
                self.gen_component_pkg_(
                    &date_dir,
                    component.name,
                    version,
                    Some(target),
                    checksums,
                )?;
            }
        }

        Ok(())
    }

    fn gen_component_pkg_(
        &self,
        date_dir: &Path,
        name: &str,
        version: &str,
        target: Option<&str>,
        checksums: &mut Vec<(String, String)>,
    ) -> Result<()> {
        let check_sum = RustInstallerPackage::new(name, version).generate(date_dir, target)?;

        let rename = RENAMES.get(name).copied().unwrap_or(name);
        let key = format!("[pkg.{rename}.target.{}]", target.unwrap_or("\"*\""));
        checksums.push((key, check_sum));
        Ok(())
    }

    /// Reads channel-rust template, and return the adjusted content.
    ///
    /// Adjusted manifest has the below modifications:
    /// - Date
    /// - Rust version
    /// - `xz_hash`s
    fn modified_channel_manifest_(
        &self,
        file_path: &Path,
        date: &str,
        version: &str,
        checksums: &[(String, String)],
    ) -> Result<String> {
        let file = fs::File::open(file_path)?;
        let reader = BufReader::new(file);

        let mut checksum_to_modify: Option<&String> = None;
        let mut lines: Vec<String> = Vec::new();

        for line in reader.lines() {
            let mut line = line?.replace("{DATE}", date).replace("{VERSION}", version);
            let trimmed = line.trim();

            if trimmed.starts_with('[') {
                for (k, v) in checksums {
                    if trimmed.starts_with(k) {
                        checksum_to_modify = Some(v);
                        break;
                    } else {
                        checksum_to_modify = None;
                    }
                }
            }

            if let Some(cs) = checksum_to_modify {
                if trimmed.starts_with("xz_hash") {
                    line = format!("xz_hash = \"{cs}\"");
                }
            }

            lines.push(line);
        }

        Ok(lines.join("\n"))
    }
}

#[derive(Debug, Default)]
/// The content of an component's package
struct RustInstallerPackage<'a> {
    name: &'a str,
    version: &'a str,
    git_commit_hash: String,
    install_script: String,
    rust_installer_version: String,
    bins: Vec<String>,
    libs: Vec<String>,
}

impl<'a> RustInstallerPackage<'a> {
    fn new(name: &'a str, version: &'a str) -> Self {
        Self {
            name,
            version,
            rust_installer_version: "3".to_string(),
            bins: vec![format!("{name}{}", std::env::consts::EXE_SUFFIX)],
            ..Default::default()
        }
    }
    /// Generate the pacakge and return the checksum of it.
    fn generate(self, root: &Path, target: Option<&str>) -> Result<String> {
        let temp_dir = tempfile::tempdir_in(root)?;
        let pkg_name = format!(
            "{}-{}{}",
            self.name,
            self.version,
            target.map(|t| format!("-{t}")).unwrap_or_default(),
        );
        let comp_name = RENAMES.get(self.name).copied().unwrap_or(self.name);

        let pre_packed = temp_dir.path().join(&pkg_name);
        let source_dir = pre_packed.join(comp_name);

        let mut manifest_in = String::new();
        let bin_paths = self.bins.iter().map(|bin| format!("bin/{bin}"));
        let lib_paths = self.libs.iter().map(|lib| format!("lib/{lib}"));

        // Create empty files for the above paths.
        // Note: We might want to generate fake binaries that can print version in the future,
        // but it doesn't seems that's needed for now.
        for path in bin_paths.chain(lib_paths) {
            let full_path = source_dir.join(&path);
            common::ensure_parent_dir(&full_path)?;
            fs::write(&full_path, "")?;
            manifest_in.push_str(&format!("file:{path}"));
        }

        // Create manifest.in
        fs::write(source_dir.join("manifest.in"), manifest_in)?;
        // Create components
        fs::write(pre_packed.join("components"), comp_name)?;
        // Create git-commit-hash
        fs::write(pre_packed.join("git-commit-hash"), self.git_commit_hash)?;
        // Create install.sh
        fs::write(pre_packed.join("install.sh"), self.install_script)?;
        // Create rust-installer-version
        fs::write(
            pre_packed.join("rust-installer-version"),
            self.rust_installer_version,
        )?;

        // Pack the `pre_packed` folder to tarball
        let tarball_name = format!("{pkg_name}.tar.xz");
        let tarball_path = root.join(&tarball_name);
        common::compress_xz(pre_packed, tarball_path)?;
        // Generate checksum
        let tarball_path = root.join(&tarball_name);
        write_checksum(&tarball_path)
    }
}

/// Write checksum to a file next to `path`, and return the calculated sha256 checksum.
fn write_checksum(path: &Path) -> Result<String> {
    let checksum = calculate_sha256(path)?;
    let mut checksum_path = path.as_os_str().to_os_string();
    checksum_path.push(".sha256");
    let filename = path
        .file_name()
        .and_then(|oss| oss.to_str())
        .ok_or_else(|| anyhow!("cannot get a valid file name of '{}'", path.display()))?;
    fs::write(checksum_path, format!("{checksum} {filename}"))?;
    Ok(checksum)
}

fn calculate_sha256(file_path: &Path) -> Result<String> {
    // Open the file
    let file = fs::File::open(file_path)?;
    let mut reader = BufReader::new(file);

    // Create a SHA-256 hasher
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0; 4096]; // Read in chunks

    // Read file in chunks and update the hash
    while let Ok(n) = reader.read(&mut buffer) {
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    // Finalize and get the checksum as a hex string
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

pub(crate) fn generate_rim_server_files() -> Result<()> {
    let mocked = RimServer::new();
    mocked.gen_toolset_manifests()?;
    mocked.gen_dist_manifest()?;
    Ok(())
}

pub(crate) fn generate_rustup_server_files(root: Option<PathBuf>) -> Result<()> {
    let mocked = RustupServer::new(root);
    mocked.gen_dist_dir()?;
    Ok(())
}
