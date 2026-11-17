use gpui::WindowAppearance;
use objc2::msg_send;

use crate::NSStringExt as _;
use crate::ns::id;

pub(crate) unsafe fn window_appearance_from_native(appearance: id) -> WindowAppearance {
    let name: id = msg_send![appearance, name];
    unsafe {
        if name == NSAppearanceNameVibrantLight {
            WindowAppearance::VibrantLight
        } else if name == NSAppearanceNameVibrantDark {
            WindowAppearance::VibrantDark
        } else if name == NSAppearanceNameAqua {
            WindowAppearance::Light
        } else if name == NSAppearanceNameDarkAqua {
            WindowAppearance::Dark
        } else {
            log::warn!("unknown appearance: {:?}", name.to_str());
            WindowAppearance::Light
        }
    }
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    pub(crate) static NSAppearanceNameAqua: id;
    pub(crate) static NSAppearanceNameDarkAqua: id;
    pub(crate) static NSAppearanceNameVibrantLight: id;
    pub(crate) static NSAppearanceNameVibrantDark: id;
}
