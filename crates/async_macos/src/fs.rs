//! Async wrappers over `std::fs` that run on GCD worker threads.

use std::fs;
use std::io;
use std::os::unix::fs::symlink as std_symlink;
use std::path::Path;
use std::path::PathBuf;

use crate::gcd::unblock;

macro_rules! wrap {
    ($(#[$meta:meta])* $name:ident($($arg:ident),+) -> $ret:ty = $call:expr) => {
        $(#[$meta])*
        pub async fn $name($($arg: impl AsRef<Path>),+) -> io::Result<$ret> {
            $(let $arg = $arg.as_ref().to_owned();)+
            unblock(move || $call).await
        }
    };
}

wrap!(metadata(path) -> fs::Metadata = fs::metadata(path));
wrap!(symlink_metadata(path) -> fs::Metadata = fs::symlink_metadata(path));
wrap!(canonicalize(path) -> PathBuf = fs::canonicalize(path));
wrap!(read_link(path) -> PathBuf = fs::read_link(path));
wrap!(read_to_string(path) -> String = fs::read_to_string(path));
wrap!(create_dir_all(path) -> () = fs::create_dir_all(path));
wrap!(remove_dir(path) -> () = fs::remove_dir(path));
wrap!(remove_dir_all(path) -> () = fs::remove_dir_all(path));
wrap!(remove_file(path) -> () = fs::remove_file(path));
wrap!(copy(from, to) -> u64 = fs::copy(from, to));
wrap!(rename(from, to) -> () = fs::rename(from, to));
wrap!(
    /// Creates a symlink at `link` pointing to `original`.
    symlink(original, link) -> () = std_symlink(original, link)
);

pub async fn write(path: impl AsRef<Path>, contents: impl Into<Vec<u8>>) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    let contents = contents.into();
    unblock(move || fs::write(path, contents)).await
}

pub async fn set_permissions(path: impl AsRef<Path>, permissions: fs::Permissions) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    unblock(move || fs::set_permissions(path, permissions)).await
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use super::*;

    #[test]
    fn round_trips_files_and_dirs() {
        block_on(async {
            let root = std::env::temp_dir().join(format!("util-async-fs-{}", std::process::id()));
            remove_dir_all(&root).await.ok();
            let nested = root.join("a/b");
            create_dir_all(&nested).await.unwrap();
            assert!(metadata(&nested).await.unwrap().is_dir());

            let file = nested.join("file.txt");
            write(&file, "hello").await.unwrap();
            assert_eq!(read_to_string(&file).await.unwrap(), "hello");

            let copied = nested.join("copy.txt");
            assert_eq!(copy(&file, &copied).await.unwrap(), 5);
            let renamed = nested.join("renamed.txt");
            rename(&copied, &renamed).await.unwrap();
            metadata(&copied).await.unwrap_err();

            let link = nested.join("link");
            symlink(&file, &link).await.unwrap();
            assert!(symlink_metadata(&link).await.unwrap().is_symlink());
            assert_eq!(read_link(&link).await.unwrap(), file);
            assert_eq!(canonicalize(&link).await.unwrap(), canonicalize(&file).await.unwrap());

            use std::os::unix::fs::PermissionsExt;
            set_permissions(&file, fs::Permissions::from_mode(0o755)).await.unwrap();
            assert_eq!(metadata(&file).await.unwrap().permissions().mode() & 0o777, 0o755);

            remove_file(&renamed).await.unwrap();
            remove_dir(&nested).await.unwrap_err();
            remove_dir_all(&root).await.unwrap();
            metadata(&root).await.unwrap_err();
        });
    }
}
