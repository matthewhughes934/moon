use crate::helpers::get_root;
use log::debug;
use proto_error::ProtoError;
use serde::Serialize;
use serde_json::Value;
use std::fmt::Write;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tinytemplate::error::Error as TemplateError;
use tinytemplate::TinyTemplate;

#[derive(Serialize)]
pub struct Context {
    bin_path: PathBuf,
    install_dir: Option<PathBuf>,
    name: String,
    parent_name: Option<String>,
    root: PathBuf,
    version: Option<String>,
}

#[async_trait::async_trait]
pub trait Shimable<'tool>: Send + Sync {
    /// Create one or many shims in the root of the tool's install directory.
    async fn create_shims(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    /// Return an absolute path to the shim file if utilizing shims.
    fn get_shim_path(&self) -> Option<&Path> {
        None
    }
}

fn format_uppercase(value: &Value, output: &mut String) -> Result<(), TemplateError> {
    if let Value::String(string) = value {
        write!(output, "{}", string.to_uppercase())?;
    }

    Ok(())
}

fn build_shim_file(builder: &ShimBuilder) -> Result<String, ProtoError> {
    let handle_error = |e: TemplateError| ProtoError::Shim(e.to_string());
    let mut template = TinyTemplate::new();

    template.add_formatter("uppercase", format_uppercase);

    template
        .add_template(
            "shim",
            if cfg!(windows) {
                include_str!("../templates/batch.tpl")
            } else {
                include_str!("../templates/bash.tpl")
            },
        )
        .map_err(handle_error)?;

    template
        .render("shim", &builder.create_context()?)
        .map_err(handle_error)
}

#[cfg(windows)]
fn get_shim_file_name(name: &str) -> String {
    format!("{}.bat", name)
}

#[cfg(not(windows))]
fn get_shim_file_name(name: &str) -> String {
    name.to_owned()
}

pub struct ShimBuilder {
    pub name: String,
    pub bin_path: PathBuf,
    pub install_dir: Option<PathBuf>,
    pub parent_name: Option<String>,
    pub version: Option<String>,
}

impl ShimBuilder {
    pub fn new(name: &str, bin_path: &Path) -> Self {
        ShimBuilder {
            name: name.to_owned(),
            bin_path: bin_path.to_path_buf(),
            install_dir: None,
            parent_name: None,
            version: None,
        }
    }

    pub fn dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.install_dir = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn parent<V: AsRef<str>>(&mut self, name: V) -> &mut Self {
        self.parent_name = Some(name.as_ref().to_owned());
        self
    }

    pub fn version<V: AsRef<str>>(&mut self, version: V) -> &mut Self {
        self.version = Some(version.as_ref().to_owned());
        self
    }

    pub fn create(&self) -> Result<PathBuf, ProtoError> {
        let shim_path = self
            .install_dir
            .as_ref()
            .unwrap()
            .join(get_shim_file_name(&self.name));

        let handle_error =
            |e: std::io::Error| ProtoError::Fs(shim_path.to_path_buf(), e.to_string());

        fs::write(&shim_path, build_shim_file(self)?).map_err(handle_error)?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&shim_path, fs::Permissions::from_mode(0o755))
                .map_err(handle_error)?;
        }

        debug!(target: "proto:shimmer", "Created shim at {}", shim_path.to_string_lossy());

        Ok(shim_path)
    }

    pub fn create_context(&self) -> Result<Context, ProtoError> {
        Ok(Context {
            bin_path: self.bin_path.clone(),
            install_dir: self.install_dir.clone(),
            name: self.name.clone(),
            parent_name: self.parent_name.clone(),
            root: get_root()?,
            version: self.version.clone(),
        })
    }
}