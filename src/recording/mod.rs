use core::ptr::null;
use x11::xlib;

pub struct Image<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,

    raw_ptr: *mut xlib::XImage,
    destroy: Option<unsafe extern "C" fn(*mut xlib::XImage) -> i32>,
}

impl Image<'_> {
    pub fn free(&mut self) {
        unsafe {
            if let Some(free) = self.destroy {
                free(self.raw_ptr);
                self.raw_ptr = null::<xlib::XImage>() as *mut xlib::XImage;
                self.data = &[];
                self.destroy = None;
            }
        }
    }
}

pub fn open_display() -> *mut xlib::_XDisplay {
    let display = unsafe { xlib::XOpenDisplay(null()) };

    display
}

pub fn record_linux(display: *mut xlib::_XDisplay, xid: u64) -> Image<'static> {
    let mut attr: xlib::XWindowAttributes = xlib::XWindowAttributes {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
        border_width: 0,
        depth: 0,
        visual: null::<xlib::Visual>() as *mut xlib::Visual,
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
        screen: null::<xlib::Screen>() as *mut xlib::Screen,
    };

    unsafe {
        xlib::XGetWindowAttributes(display, xid, core::ptr::addr_of_mut!(attr));
    }

    let width = attr.width;
    let height = attr.height;
    let image = unsafe {
        xlib::XGetImage(
            display,
            xid,
            0,
            0,
            width as u32,
            height as u32,
            0xffffffff,
            xlib::ZPixmap,
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
