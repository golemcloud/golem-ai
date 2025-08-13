use crate::golem::exec::types::Error;
use rquickjs::{Ctx, JsLifetime};
use std::path::{Path, PathBuf};

#[rquickjs::module(rename_vars = "camelCase")]
pub mod native_module {
    use encoding_rs::Encoding;
    use rquickjs::prelude::List;
    use rquickjs::{Ctx, TypedArray};

    #[rquickjs::function]
    pub fn read_file_with_encoding(
        path: String,
        encoding: String,
        ctx: Ctx<'_>,
    ) -> List<(Option<String>, Option<String>)> {
        let path = super::resolve_path(&ctx, &path);
        match std::fs::read(&path) {
            Ok(bytes) => match Encoding::for_label(encoding.as_bytes()) {
                Some(encoding) => {
                    let (decoded, _) = encoding.decode_with_bom_removal(&bytes);
                    let decoded_string = decoded.into_owned();
                    List((Some(decoded_string), None))
                }
                None => List((None, Some(format!("Unsupported encoding: {encoding}")))),
            },
            Err(err) => {
                let error_message = format!("Failed to read file {path:?}: {err}");
                List((None, Some(error_message)))
            }
        }
    }

    #[rquickjs::function]
    pub fn read_file(
        path: String,
        ctx: Ctx<'_>,
    ) -> List<(Option<TypedArray<'_, u8>>, Option<String>)> {
        let path = super::resolve_path(&ctx, &path);
        match std::fs::read(&path) {
            Ok(bytes) => {
                let typed_array =
                    TypedArray::new_copy(ctx.clone(), &bytes).expect("Failed to create TypedArray");
                List((Some(typed_array), None))
            }
            Err(err) => {
                let error_message = format!("Failed to read file {path:?}: {err}");
                List((None, Some(error_message)))
            }
        }
    }

    #[rquickjs::function]
    pub fn write_file_with_encoding(
        path: String,
        encoding: String,
        content: String,
        ctx: Ctx<'_>,
    ) -> Option<String> {
        if encoding != "utf8" {
            Some("Only 'utf8' encoding is supported".to_string())
        } else {
            let bytes = content.as_bytes();

            if let Ok(Some(limit)) = ctx
                .globals()
                .get::<&str, Option<u64>>("__golem_exec_js_file_size_limit")
            {
                if bytes.len() as u64 > limit {
                    return Some(format!(
                        "File size {} exceeds the limit of {} bytes",
                        bytes.len(),
                        limit
                    ));
                }
            }

            let path = super::resolve_path(&ctx, &path);
            if let Some(parent) = path.parent() {
                if let Err(err) = std::fs::create_dir_all(parent) {
                    return Some(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        err
                    ));
                }
            }
            if let Err(err) = std::fs::write(&path, bytes) {
                Some(format!("Failed to write file {path:?}: {err}"))
            } else {
                None // Success
            }
        }
    }

    #[rquickjs::function]
    pub fn write_file(path: String, content: TypedArray<'_, u8>, ctx: Ctx<'_>) -> Option<String> {
        if let Some(bytes) = content.as_bytes() {
            if let Ok(Some(limit)) = ctx
                .globals()
                .get::<&str, Option<u64>>("__golem_exec_js_file_size_limit")
            {
                if bytes.len() as u64 > limit {
                    return Some(format!(
                        "File size {} exceeds the limit of {} bytes",
                        bytes.len(),
                        limit
                    ));
                }
            }

            let path = super::resolve_path(&ctx, &path);
            if let Some(parent) = path.parent() {
                if let Err(err) = std::fs::create_dir_all(parent) {
                    return Some(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        err
                    ));
                }
            }
            if let Err(err) = std::fs::write(&path, bytes) {
                Some(format!("Failed to write file {path:?}: {err}"))
            } else {
                None // Success
            }
        } else {
            Some("The typed array has been detached".to_string())
        }
    }
}

#[derive(Debug, Clone, JsLifetime)]
struct FsConfig {
    pub data_root: PathBuf,
}

fn get_data_root(ctx: &Ctx<'_>) -> PathBuf {
    let fs_config = ctx.userdata::<FsConfig>().unwrap();
    fs_config.data_root.clone()
}

fn resolve_path(ctx: &Ctx<'_>, path: &str) -> PathBuf {
    let data_root = get_data_root(ctx);
    let resolved_path = if let Some(stripped) = path.strip_prefix('/') {
        data_root.join(stripped)
    } else {
        let cwd: String = ctx
            .globals()
            .get("__golem_exec_js_cwd")
            .expect("Failed to get cwd");
        let cwd = cwd.trim_start_matches('/');
        data_root.join(cwd).join(path)
    };

    if !resolved_path.starts_with(&data_root) {
        panic!("Path {resolved_path:?} is outside the data root {data_root:?}",);
    }

    resolved_path
}

pub fn init_fs(ctx: Ctx<'_>, data_root: &Path) -> Result<(), Error> {
    let fs_config = FsConfig {
        data_root: data_root.to_path_buf(),
    };
    ctx.store_userdata(fs_config)
        .map_err(|err| Error::Internal(err.to_string()))?;
    Ok(())
}

// JS functions for the fs implementation
pub const FS_JS: &str = include_str!("fs.js");
