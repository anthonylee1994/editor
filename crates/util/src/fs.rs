use std::fs::Permissions;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Set the permissions for the given path so that the file becomes executable.
pub async fn make_file_executable(path: &Path) -> io::Result<()> {
    async_macos::fs::set_permissions(path, Permissions::from_mode(0o755)).await
}
