use core::ptr::null;

#[cfg(target_os = "linux")]
pub struct Image<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,

    raw_ptr: *mut x11::xlib::XImage,
    destroy: Option<unsafe extern "C" fn(*mut x11::xlib::XImage) -> i32>,
}

#[cfg(target_os = "linux")]
impl Image<'_> {
    pub fn free(&mut self) {
        unsafe {
            if let Some(free) = self.destroy {
                free(self.raw_ptr);
                self.raw_ptr = null::<x11::xlib::XImage>() as *mut x11::xlib::XImage;
                self.data = &[];
                self.destroy = None;
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub fn open_display() -> *mut x11::xlib::_XDisplay {
    let display = unsafe { x11::xlib::XOpenDisplay(null()) };

    display
}

#[cfg(target_os = "linux")]
pub fn record_linux(display: *mut x11::xlib::_XDisplay, xid: u64) -> Image<'static> {
    let mut attr: x11::xlib::XWindowAttributes = x11::xlib::XWindowAttributes {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
        border_width: 0,
        depth: 0,
        visual: null::<x11::xlib::Visual>() as *mut x11::xlib::Visual,
        root: 0,
        class: 0,
        bit_gravity: 0,
        win_gravity: 0,
        backing_store: 0,
        backing_planes: 0,
        backing_pixel: 0,
        save_under: 0,
        colormap: 0,
        map_installed: 0,
        map_state: 0,
        all_event_masks: 0,
        your_event_mask: 0,
        do_not_propagate_mask: 0,
        override_redirect: 0,
        screen: null::<x11::xlib::Screen>() as *mut x11::xlib::Screen,
    };

    unsafe {
        x11::xlib::XGetWindowAttributes(display, xid, core::ptr::addr_of_mut!(attr));
    }

    let width = attr.width;
    let height = attr.height;
    let image = unsafe {
        x11::xlib::XGetImage(
            display,
            xid,
            0,
            0,
            width as u32,
            height as u32,
            0xffffffff,
            x11::xlib::ZPixmap,
        )
    };

    let slice = unsafe {
        std::slice::from_raw_parts((*image).data as *const u8, (width * height * 4) as usize)
    };

    let mut img: Image = Image {
        data: slice,
        destroy: None,
        raw_ptr: image,
        width: attr.width as u32,
        height: attr.height as u32,
    };

    unsafe {
        if let Some(destroy_image) = (*image).funcs.destroy_image {
            img.destroy = Some(destroy_image);
        }
    }

    img
}
