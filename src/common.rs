use once_cell::sync::Lazy;
use proc_macro2::Span;
use std::env;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

pub static TOKIO_RT: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to start Tokio runtime")
});

pub fn block_on<F>(f: F) -> F::Output
where
    F: std::future::Future,
{
    TOKIO_RT.block_on(f)
}

pub fn hash_string(query: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(query.as_bytes()))
}

pub fn resolve_path(path: impl AsRef<Path>, err_span: Span) -> syn::Result<PathBuf> {
    let path = path.as_ref();

    if path.is_absolute() {
        return Err(syn::Error::new(
            err_span,
            "absolute paths will only work on the current machine",
        ));
    }

    let base_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        syn::Error::new(
            err_span,
            "CARGO_MANIFEST_DIR is not set; please use Cargo to build",
        )
    })?;
    let base_dir_path = Path::new(&base_dir);

    Ok(base_dir_path.join(path))
}
