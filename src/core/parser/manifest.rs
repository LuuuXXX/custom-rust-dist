#![allow(unused)]

use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::{collections::BTreeMap, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use url::Url;

use crate::core::install::InstallConfiguration;
use crate::utils;

use super::TomlParser;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct ToolsetManifest {
    pub(crate) rust: RustToolchain,
    #[serde(default)]
    pub(crate) tools: Tools,
    /// Path to the manifest file.
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl TomlParser for ToolsetManifest {
    fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let raw = utils::read_to_string(&path)?;
        let mut temp_manifest = Self::from_str(&raw)?;
        temp_manifest.path = Some(path.as_ref().to_path_buf());
        Ok(temp_manifest)
    }
}

impl ToolsetManifest {
    pub fn toolchain_components(&self) -> Vec<&str> {
        self.rust
            .components
            .as_ref()
            .map(|seq| seq.iter().map(Deref::deref).collect())
            .unwrap_or_default()
    }

    pub fn get_tool_description(&self, toolname: &str) -> Option<&str> {
        self.tools.descriptions.get(toolname).map(|s| s.as_str())
    }

    /// Get the group name of a certain tool, if exist.
    pub fn group_name(&self, toolname: &str) -> Option<&str> {
        for (group, tools) in &self.tools.group {
            if tools.contains(toolname) {
                return Some(group);
            }
        }
        None
    }

    /// Get a map of [`Tool`] that are available only in current target.
    pub fn current_target_tools(&self) -> BTreeMap<&String, &ToolInfo> {
        let cur_target = env!("TARGET");
        // Clippy bug, the `map(|(k, v)| (k, v))` cannot be removed
        #[allow(clippy::map_identity)]
        self.tools
            .target
            .get(cur_target)
            .map(|map| map.iter().map(|(k, v)| (k, v)).collect())
            .unwrap_or_default()
    }

    /// Get a mut reference to the map of [`Tool`] that are available only in current target.
    ///
    /// Return `None` if there are no available tools in the current target.
    pub fn current_target_tools_mut(&mut self) -> Option<&mut BTreeMap<String, ToolInfo>> {
        let cur_target = env!("TARGET");
        // Clippy bug, the `map(|(k, v)| (k, v))` cannot be removed
        #[allow(clippy::map_identity)]
        self.tools.target.get_mut(cur_target)
    }

    /// Turn all the relative paths in the `tools` section to some absolute paths.
    ///
    /// There are some rules applied when converting, including:
    /// 1. If the manifest was loaded from a path,
    ///     all relative paths will be forced to combine with the path loading from.
    /// 2. If the manifest was not loaded from path,
    ///     all relative paths will be forced to combine with the parent directory of this executable.
    ///     (Assuming the manifest was baked in the executable)
    ///
    /// # Errors
    /// Return `Result::Err` if the manifest was not loaded from path, and the current executable path
    /// cannot be determined as well.
    pub fn adjust_paths(&mut self) -> anyhow::Result<()> {
        let parent_dir = if let Some(p) = &self.path {
            p.to_path_buf()
        } else {
            std::env::current_exe()?
                .parent()
                .unwrap_or_else(|| unreachable!("an executable always have a parent directory"))
                .to_path_buf()
        };

        for tool in self.tools.target.values_mut() {
            for tool_info in tool.values_mut() {
                if let ToolInfo::Path { path, .. } = tool_info {
                    *path = utils::to_nomalized_abspath(path.as_path(), Some(&parent_dir))?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct RustToolchain {
    pub(crate) version: String,
    pub(crate) profile: Option<String>,
    pub(crate) components: Option<Vec<String>>,
}

impl RustToolchain {
    pub(crate) fn new(ver: &str) -> Self {
        Self {
            version: ver.to_string(),
            profile: None,
            components: None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Default)]
pub(crate) struct Tools {
    #[serde(default)]
    descriptions: BTreeMap<String, String>,
    /// Containing groups of tools.
    ///
    /// Note that not all tools will have a group.
    #[serde(default)]
    group: BTreeMap<String, HashSet<String>>,
    #[serde(default)]
    target: BTreeMap<String, BTreeMap<String, ToolInfo>>,
}

impl Tools {
    pub(crate) fn new<I>(targeted_tools: I) -> Tools
    where
        I: IntoIterator<Item = (String, BTreeMap<String, ToolInfo>)>,
    {
        Self {
            descriptions: BTreeMap::default(),
            group: BTreeMap::default(),
            target: BTreeMap::from_iter(targeted_tools),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum ToolInfo {
    // FIXME (?): Currently there's no way to specify a tool with plain version to be required.
    PlainVersion(String),
    Git {
        git: Url,
        branch: Option<String>,
        tag: Option<String>,
        rev: Option<String>,
        #[serde(default)]
        required: bool,
    },
    Path {
        path: PathBuf,
        version: Option<String>,
        #[serde(default)]
        required: bool,
    },
    Url {
        url: Url,
        version: Option<String>,
        #[serde(default)]
        required: bool,
    },
}

impl ToolInfo {
    pub fn is_required(&self) -> bool {
        match self {
            Self::PlainVersion(_) => false,
            Self::Git { required, .. }
            | Self::Path { required, .. }
            | Self::Url { required, .. } => *required,
        }
    }

    pub fn convert_to_path(&mut self, path: PathBuf) {
        match self {
            Self::PlainVersion(ver) => {
                *self = Self::Path {
                    path,
                    version: Some(ver.to_owned()),
                    required: false,
                };
            }
            Self::Git { required, .. } => {
                *self = Self::Path {
                    path,
                    version: None,
                    required: *required,
                };
            }
            Self::Path {
                version, required, ..
            }
            | Self::Url {
                version, required, ..
            } => {
                *self = Self::Path {
                    path,
                    version: version.to_owned(),
                    required: *required,
                };
            }
        }
    }
}

pub fn baked_in_manifest() -> Result<ToolsetManifest> {
    ToolsetManifest::from_str(include_str!("../../../resources/toolset_manifest.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenient macro to initialize **Non-Required** `ToolInfo`
    macro_rules! tool_info {
        ($version:literal) => {
            ToolInfo::PlainVersion($version.into())
        };
        ($url_str:literal, $version:expr) => {
            ToolInfo::Url {
                version: $version.map(ToString::to_string),
                url: $url_str.parse().unwrap(),
                required: false,
            }
        };
        ($git:literal, $branch:expr, $tag:expr, $rev:expr) => {
            ToolInfo::Git {
                git: $git.parse().unwrap(),
                branch: $branch.map(ToString::to_string),
                tag: $tag.map(ToString::to_string),
                rev: $rev.map(ToString::to_string),
                required: false,
            }
        };
        ($path:expr, $version:expr) => {
            ToolInfo::Path {
                version: $version.map(ToString::to_string),
                path: $path,
                required: false,
            }
        };
    }

    #[test]
    fn deserialize_minimal_manifest() {
        let input = r#"
[rust]
version = "1.0.0"
"#;
        assert_eq!(
            ToolsetManifest::from_str(input).unwrap(),
            ToolsetManifest {
                rust: RustToolchain::new("1.0.0"),
                tools: Tools::default(),
                path: None,
            }
        )
    }

    #[test]
    fn deserialize_complicated_manifest() {
        let input = r#"
[rust]
version = "1.0.0"
profile = "minimal"
components = ["clippy-preview", "llvm-tools-preview"]

[tools.target.x86_64-pc-windows-msvc]
t1 = "0.1.0" # use cargo install
t2 = { path = "/path/to/local" }
t3 = { url = "https://example.com/path/to/tool" }

[tools.target.x86_64-unknown-linux-gnu]
t1 = "0.1.0" # use cargo install
t2 = { path = "/path/to/local" }

[tools.target.aarch64-unknown-linux-gnu]
t1 = "0.1.0"
t4 = { git = "https://git.example.com/org/tool", branch = "stable" }
"#;

        let mut x86_64_windows_msvc_tools = BTreeMap::new();
        x86_64_windows_msvc_tools.insert("t1".to_string(), tool_info!("0.1.0"));
        x86_64_windows_msvc_tools.insert(
            "t2".to_string(),
            tool_info!(PathBuf::from("/path/to/local"), None::<&str>),
        );
        x86_64_windows_msvc_tools.insert(
            "t3".to_string(),
            tool_info!("https://example.com/path/to/tool", None::<&str>),
        );

        let mut x86_64_linux_gnu_tools = BTreeMap::new();
        x86_64_linux_gnu_tools.insert("t1".to_string(), tool_info!("0.1.0"));
        x86_64_linux_gnu_tools.insert(
            "t2".to_string(),
            tool_info!(PathBuf::from("/path/to/local"), None::<&str>),
        );

        let mut aarch64_linux_gnu_tools = BTreeMap::new();
        aarch64_linux_gnu_tools.insert("t1".to_string(), tool_info!("0.1.0"));
        aarch64_linux_gnu_tools.insert(
            "t4".to_string(),
            tool_info!(
                "https://git.example.com/org/tool",
                Some("stable"),
                None::<&str>,
                None::<&str>
            ),
        );

        let expected = ToolsetManifest {
            rust: RustToolchain {
                version: "1.0.0".into(),
                profile: Some("minimal".into()),
                components: Some(vec!["clippy-preview".into(), "llvm-tools-preview".into()]),
            },
            tools: Tools::new([
                (
                    "x86_64-pc-windows-msvc".to_string(),
                    x86_64_windows_msvc_tools,
                ),
                (
                    "x86_64-unknown-linux-gnu".to_string(),
                    x86_64_linux_gnu_tools,
                ),
                (
                    "aarch64-unknown-linux-gnu".to_string(),
                    aarch64_linux_gnu_tools,
                ),
            ]),
            path: None,
        };

        assert_eq!(ToolsetManifest::from_str(input).unwrap(), expected);
    }

    #[test]
    fn deserialize_realworld_manifest() {
        let input = include_str!("../../../tests/data/toolset_manifest.toml");
        let expected = ToolsetManifest {
            rust: RustToolchain {
                version: "stable".into(),
                profile: Some("minimal".into()),
                components: Some(vec!["clippy-preview".into(), "rustfmt".into()]),
            },
            tools: Tools::new([
                (
                    "x86_64-pc-windows-msvc".into(),
                    BTreeMap::from_iter([
                        ("buildtools".to_string(), tool_info!(PathBuf::from("tests/cache/BuildTools-With-SDK.zip"), Some("1"))),
                        ("cargo-llvm-cov".to_string(), tool_info!("https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.6.11/cargo-llvm-cov-x86_64-pc-windows-msvc.zip", Some("0.6.11"))),
                        ("vscode".to_string(), tool_info!(PathBuf::from("tests/cache/VSCode-win32-x64-1.91.1.zip"), Some("1.91.1"))),
                        ("vscode-rust-analyzer".to_string(), tool_info!(PathBuf::from("tests/cache/rust-lang.rust-analyzer-0.4.2054@win32-x64.vsix"), Some("0.4.2054"))),
                        ("cargo-expand".to_string(), tool_info!("1.0.88")),
                    ]),
                ),
                (
                    "x86_64-pc-windows-gnu".into(),
                    BTreeMap::from_iter([
                        ("mingw64".to_string(), tool_info!(PathBuf::from("tests/cache/x86_64-13.2.0-release-posix-seh-msvcrt-rt_v11-rev1.7z"), Some("13.2.0"))),
                        ("vscode".to_string(), tool_info!(PathBuf::from("tests/cache/VSCode-win32-x64-1.91.1.zip"), Some("1.91.1"))),
                        ("vscode-rust-analyzer".to_string(), tool_info!(PathBuf::from("tests/cache/rust-lang.rust-analyzer-0.4.2054@win32-x64.vsix"), Some("0.4.2054"))),
                        ("cargo-expand".to_string(), tool_info!("1.0.88")),
                    ]),
                ),
                (
                    "x86_64-unknown-linux-gnu".into(),
                    BTreeMap::from_iter([
                        ("cargo-llvm-cov".to_string(), tool_info!("https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.6.11/cargo-llvm-cov-x86_64-unknown-linux-gnu.tar.gz", Some("0.6.11"))),
                        ("flamegraph".to_string(), tool_info!("https://github.com/flamegraph-rs/flamegraph", None::<&str>, Some("v0.6.5"), None::<&str>)),
                        ("cargo-expand".to_string(), tool_info!("1.0.88")),
                    ]),
                ),
                (
                    "aarch64-apple-darwin".into(),
                    BTreeMap::from_iter([
                        ("cargo-llvm-cov".to_string(), tool_info!("https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.6.11/cargo-llvm-cov-aarch64-apple-darwin.tar.gz", Some("0.6.11"))),
                        ("flamegraph".to_string(), tool_info!("https://github.com/flamegraph-rs/flamegraph", None::<&str>, Some("v0.6.5"), None::<&str>)),
                        ("cargo-expand".to_string(), tool_info!("1.0.88")),
                    ]),
                ),
            ]),
            path: None,
        };
        assert_eq!(ToolsetManifest::from_str(input).unwrap(), expected);
    }

    #[test]
    fn current_target_tools_are_correct() {
        let input = include_str!("../../../tests/data/toolset_manifest.toml");
        let manifest = ToolsetManifest::from_str(input).unwrap();
        let tools = manifest.current_target_tools();

        #[cfg(all(windows, target_env = "gnu"))]
        assert_eq!(
            tools,
            BTreeMap::from([
                (
                    &"mingw64".into(),
                    &tool_info!(
                        PathBuf::from(
                            "tests/cache/x86_64-13.2.0-release-posix-seh-msvcrt-rt_v11-rev1.7z"
                        ),
                        Some("13.2.0")
                    )
                ),
                (
                    &"vscode".into(),
                    &tool_info!(
                        PathBuf::from("tests/cache/VSCode-win32-x64-1.91.1.zip"),
                        Some("1.91.1")
                    )
                ),
                (
                    &"vscode-rust-analyzer".into(),
                    &tool_info!(
                        PathBuf::from(
                            "tests/cache/rust-lang.rust-analyzer-0.4.2054@win32-x64.vsix"
                        ),
                        Some("0.4.2054")
                    )
                ),
                (&"cargo-expand".into(), &tool_info!("1.0.88")),
            ])
        );

        #[cfg(all(windows, target_env = "msvc"))]
        assert_eq!(
            tools,
            BTreeMap::from([
                (
                    &"buildtools".into(),
                    &tool_info!(
                        "tests/cache/BuildTools-With-SDK.zip".into(),
                        Some("1")
                    )
                ),
                (
                    &"cargo-llvm-cov".into(),
                    &tool_info!(
                        "https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.6.11/cargo-llvm-cov-x86_64-pc-windows-msvc.zip",
                        Some("0.6.11")
                    )
                ),
                (
                    &"vscode".into(),
                    &tool_info!(
                        "tests/cache/VSCode-win32-x64-1.91.1.zip".into(),
                        Some("1.91.1")
                    )
                ),
                (
                    &"vscode-rust-analyzer".into(),
                    &tool_info!(
                        "tests/cache/rust-lang.rust-analyzer-0.4.2054@win32-x64.vsix".into(),
                        Some("0.4.2054")
                    )
                ),
                (
                    &"cargo-expand".into(),
                    &tool_info!("1.0.88"),
                ),
            ])
        );

        #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
        assert_eq!(tools, BTreeMap::from([
            (&"cargo-llvm-cov".into(), &tool_info!("https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.6.11/cargo-llvm-cov-x86_64-unknown-linux-gnu.tar.gz", Some("0.6.11"))),
            (&"flamegraph".into(), &tool_info!("https://github.com/flamegraph-rs/flamegraph", None::<&str>, Some("v0.6.5"), None::<&str>)),
            (&"cargo-expand".into(), &tool_info!("1.0.88")),
        ]));

        // TODO: Add test for macos.
    }

    #[test]
    fn with_tools_descriptions() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.descriptions]
t1 = "desc for t1"
# t2 does not have desc
t3 = "desc for t3"
t4 = "desc for t4 that might not exist"

[tools.target.x86_64-pc-windows-msvc]
t1 = "0.1.0" # use cargo install
t2 = { path = "/path/to/local" }
t3 = { url = "https://example.com/path/to/tool" }
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();

        assert_eq!(
            expected.tools.descriptions,
            BTreeMap::from_iter([
                ("t1".to_string(), "desc for t1".to_string()),
                ("t3".to_string(), "desc for t3".to_string()),
                (
                    "t4".to_string(),
                    "desc for t4 that might not exist".to_string()
                ),
            ])
        );
    }

    #[test]
    fn with_required_property() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
t1 = "0.1.0" # use cargo install
t2 = { path = "/path/to/local", required = true }
t3 = { url = "https://example.com/path/to/tool", required = true }
t4 = { git = "https://git.example.com/org/tool", branch = "stable", required = true }
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        let tools = expected.tools.target.get("x86_64-pc-windows-msvc").unwrap();
        assert!(!tools.get("t1").unwrap().is_required());
        assert!(tools.get("t2").unwrap().is_required());
        assert!(tools.get("t3").unwrap().is_required());
        assert!(tools.get("t4").unwrap().is_required());
    }

    #[test]
    fn with_tools_group() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.group]
"Some Group" = [ "t1", "t2" ]
Others = [ "t3", "t4" ]
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        assert_eq!(
            expected.tools.group,
            BTreeMap::from_iter([
                (
                    "Some Group".to_string(),
                    ["t1".to_string(), "t2".to_string()].into_iter().collect()
                ),
                (
                    "Others".to_string(),
                    ["t3".to_string(), "t4".to_string()].into_iter().collect()
                )
            ])
        );
        assert_eq!(expected.group_name("t3"), Some("Others"));
        assert_eq!(expected.group_name("t1"), Some("Some Group"));
        assert_eq!(expected.group_name("t100"), None);
    }
}