mod icons;
mod material_folders;

use std::borrow::Cow;
use std::path::Path;
use std::str;

use gpui::App;
use gpui::SharedString;
use util::paths::PathExt as _;

use crate::icons::ICONS;

/// File and folder icons for panels and pickers, from the built-in set in `icons.rs`.
#[derive(Debug)]
pub struct FileIcons;

impl FileIcons {
    pub fn get_icon(path: &Path, cx: &App) -> Option<SharedString> {
        Self::get_icon_for_type(icon_type(path), cx)
    }

    /// Polychrome Material icon for the path; `Icon::from_path` draws it in its own colors.
    pub fn get_material_icon(path: &Path) -> SharedString {
        let icons = &ICONS.material_icons;
        icons.get(icon_type(path)).unwrap_or_else(|| &icons["default"]).clone()
    }

    pub fn get_material_folder_icon(expanded: bool, path: &Path) -> SharedString {
        let named = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| ICONS.material_named_directories.get(&*material_folder_key(name)));
        named.unwrap_or(&ICONS.material_directory).get(expanded)
    }

    pub fn get_icon_for_type(typ: &str, _cx: &App) -> Option<SharedString> {
        ICONS.file_icons.get(typ).cloned()
    }

    pub fn get_folder_icon(expanded: bool, path: &Path, _cx: &App) -> Option<SharedString> {
        let named = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .and_then(|name| ICONS.named_directories.get(name));
        Some(named.unwrap_or(&ICONS.directory).get(expanded))
    }

    pub fn get_chevron_icon(expanded: bool, _cx: &App) -> Option<SharedString> {
        Some(ICONS.chevron.get(expanded))
    }
}

/// Material matches folder names case-insensitively, with an optional `.`, `_` or `-`
/// prefix or `__name__` wrapping.
fn material_folder_key(name: &str) -> Cow<'_, str> {
    let name = match name.strip_prefix("__").and_then(|name| name.strip_suffix("__")) {
        Some(inner) if !inner.is_empty() => inner,
        _ => name.strip_prefix(['.', '_', '-']).unwrap_or(name),
    };
    if name.bytes().any(|b| b.is_ascii_uppercase()) {
        Cow::Owned(name.to_ascii_lowercase())
    } else {
        Cow::Borrowed(name)
    }
}

/// Icon key for the path, or `default`. Only keys with a built-in icon match, so an
/// unmapped key keeps searching shorter suffixes.
fn icon_type(path: &Path) -> &'static str {
    let get_icon_from_suffix = |suffix: &str| -> Option<&'static str> {
        let typ = ICONS.file_stems.get(suffix).or_else(|| ICONS.file_suffixes.get(suffix))?;
        ICONS.file_icons.contains_key(typ).then_some(typ.as_str())
    };

    if let Some(mut typ) = path.file_name().and_then(|typ| typ.to_str()) {
        // check if file name is in suffixes
        // e.g. catch file named `eslint.config.js` instead of `.eslint.config.js`
        if let Some(found) = get_icon_from_suffix(typ) {
            return found;
        }

        let mut first_prefix = None;
        while let Some((prefix, suffix)) = typ.split_once('.') {
            first_prefix.get_or_insert(prefix);
            if let Some(found) = get_icon_from_suffix(suffix) {
                return found;
            }
            typ = suffix;
        }
        // check if stem (before first dot) is in stems
        // e.g. catch file named `Dockerfile.prod` or `Podfile.lock`
        if let Some(found) = first_prefix.and_then(get_icon_from_suffix) {
            return found;
        }
    }

    // handle cases where the file extension is made up of multiple important
    // parts (e.g Component.stories.tsx) that refer to an alternative icon style
    if let Some(found) = path.multiple_extensions().and_then(|suffix| get_icon_from_suffix(suffix.as_str())) {
        return found;
    }

    // primary case: check if the files extension or the hidden file name
    // matches some icon path
    if let Some(found) = path.extension_or_hidden_file_name().and_then(get_icon_from_suffix) {
        return found;
    }

    // this _should_ only happen when the file is hidden (has leading '.')
    // and is not a "special" file we have an icon (e.g. not `.eslint.config.js`)
    // that should be caught above. In the remaining cases, we want to check
    // for a normal supported extension e.g. `.data.json` -> `json`
    path.extension().and_then(|ext| ext.to_str()).and_then(get_icon_from_suffix).unwrap_or("default")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_type_matches_names_and_extensions() {
        assert_eq!(icon_type(Path::new("src/main.rs")), "rust");
        assert_eq!(icon_type(Path::new("eslint.config.js")), "eslint");
        assert_eq!(icon_type(Path::new("Dockerfile.prod")), "docker");
        assert_eq!(icon_type(Path::new(".data.json")), "json");
        // `backup` has no built-in icon, so `.bak` falls through to the default.
        assert_eq!(icon_type(Path::new("notes.bak")), "default");
        assert_eq!(icon_type(Path::new("README")), "default");
    }

    #[test]
    fn material_icon_falls_back_to_default() {
        assert_eq!(FileIcons::get_material_icon(Path::new("main.rs")).as_ref(), "icons/material/rust.svg");
        assert_eq!(FileIcons::get_material_icon(Path::new("a.kdl")).as_ref(), "icons/material/file.svg");
        assert_eq!(FileIcons::get_material_icon(Path::new("README")).as_ref(), "icons/material/file.svg");
    }

    #[test]
    fn material_folder_icon_follows_name_and_expansion() {
        let icon = |expanded, path| FileIcons::get_material_folder_icon(expanded, Path::new(path));
        assert_eq!(icon(false, "a/unknown").as_ref(), "icons/material/folder.svg");
        assert_eq!(icon(true, "a/unknown").as_ref(), "icons/material/folder-open.svg");
        assert_eq!(icon(false, "crates/src").as_ref(), "icons/material/folder-src.svg");
        assert_eq!(icon(true, "crates/src").as_ref(), "icons/material/folder-src-open.svg");
        assert_eq!(icon(false, ".github").as_ref(), "icons/material/folder-github.svg");
        assert_eq!(icon(false, "META-INF").as_ref(), icon(false, "meta-inf").as_ref());
    }

    #[test]
    fn material_folder_key_normalizes_names() {
        assert_eq!(material_folder_key("src"), "src");
        assert_eq!(material_folder_key(".Git"), "git");
        assert_eq!(material_folder_key("_build"), "build");
        assert_eq!(material_folder_key("-tests"), "tests");
        assert_eq!(material_folder_key("__Tests__"), "tests");
        assert_eq!(material_folder_key("____"), "___");
    }

    #[test]
    fn material_icon_assets_exist() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let folders = [&ICONS.material_directory.collapsed, &ICONS.material_directory.expanded];
        let named = ICONS.material_named_directories.values().flat_map(|icons| [&icons.collapsed, &icons.expanded]);
        for path in ICONS.material_icons.values().chain(folders).chain(named) {
            assert!(assets.join(path.as_ref()).is_file(), "missing {path}");
        }
    }
}
