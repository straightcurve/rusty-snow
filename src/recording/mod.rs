use core::ptr::null;

#[cfg(target_os = "linux")]
pub struct Image {
    //pub data: Option<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>>,
    //pub data: Option<image::DynamicImage>,
    pub data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

#[cfg(target_os = "linux")]
pub fn open_display() -> *mut x11::xlib::_XDisplay {
    let display = unsafe { x11::xlib::XOpenDisplay(null()) };

    display
}

#[cfg(target_os = "linux")]
pub fn record_linux(display: *mut x11::xlib::_XDisplay, xid: u64) -> Image {
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
        data: None,
        width: attr.width as u32,
        height: attr.height as u32,
    };

    /*
    let mut _zzz = image::ImageBuffer::from_fn(img.width, img.height, |x, y| unsafe {
        if let Some(get_pixel) = (*image).funcs.get_pixel {
            let p = get_pixel(image, x as i32, y as i32);
            let s = bincode::serialize(&p).unwrap();

            /*
            println!("p: {:?}", bincode::serialize(&p).unwrap());
            println!("r: {:?}", bincode::serialize(&r).unwrap());
            println!("g: {:?}", bincode::serialize(&g).unwrap());
            println!("b: {:?}", bincode::serialize(&b).unwrap());
            println!("p & r << 2: {:?}", bincode::serialize(&(p & r)).unwrap()[2]);
            println!("p & g << 1: {:?}", bincode::serialize(&(p & g)).unwrap()[1]);
            println!("p & b << 0: {:?}", bincode::serialize(&(p & b)).unwrap()[0]);
            println!(
                "pixel {}x{} ({}, {}, {})",
                x,
                y,
                ((p & r) >> 3) as u8,
                ((p & g) >> 2) as u8,
                ((p & b) >> 1) as u8
            );
            */

            return image::Rgb([s[0], s[1], s[2]]);
        } else {
            return image::Rgb([0, 0, 0]);
        }
    });

    */
    //let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_EXT_BGRA);
    let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_EXT_RGBA);

    comp.set_size(img.width as usize, img.height as usize);
    comp.set_mem_dest();
    comp.start_compress();

    // replace with your image data
    let pixels = slice.to_vec();
    /*
    println!(
        "pixels: {}, first [{:?} ..]",
        slice.len(),
        pixels[0..=8].to_vec(),
    );
    */

    assert!(comp.write_scanlines(&pixels[..]));

    comp.finish_compress();
    let jpeg_bytes = comp.data_to_vec().unwrap();
    // write to file, etc.

    /*
    println!("pre {}x{}", _zzz.width(), _zzz.height());
    _zzz = image::imageops::resize(
        &_zzz,
        _zzz.width() / 2,
        _zzz.height() / 2,
        image::imageops::FilterType::Lanczos3,
    );
    println!("post {}x{}", _zzz.width(), _zzz.height());
    */

    img.data = Some(jpeg_bytes);

    unsafe {
        if let Some(destroy_image) = (*image).funcs.destroy_image {
            destroy_image(image);
        }
    }

    img
}
