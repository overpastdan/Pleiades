fn main() {
    println!("cargo:rerun-if-changed=assets/icon.png");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        generate_ico();

        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().expect("failed to compile windows resources");
    }
}

fn generate_ico() {
    let img = image::open("assets/icon.png")
        .expect("failed to open assets/icon.png")
        .into_rgba8();
    let img = trim_and_square(&img);

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16u32, 32, 48, 64, 128, 256] {
        let resized = image::imageops::resize(&img, size, size, image::imageops::FilterType::Lanczos3);
        let icon_image = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
        icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image).expect("failed to encode icon frame"));
    }

    let file = std::fs::File::create("assets/icon.ico").expect("failed to create assets/icon.ico");
    icon_dir.write(file).expect("failed to write assets/icon.ico");
}

/// Crops an image to the bounding box of its non-transparent pixels, then
/// pads that onto a square transparent canvas (with a small margin) so the
/// artwork fills the icon instead of leaving large empty borders.
fn trim_and_square(img: &image::RgbaImage) -> image::RgbaImage {
    let (w, h) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            if img.get_pixel(x, y)[3] > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    let trimmed = image::imageops::crop_imm(img, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
        .to_image();

    // Cap the aspect ratio so a wide/tall source doesn't leave large empty
    // bars once padded to a square icon.
    const MAX_RATIO: f32 = 1.15;
    let (tw, th) = trimmed.dimensions();
    let trimmed = if tw as f32 > th as f32 * MAX_RATIO {
        let new_w = (th as f32 * MAX_RATIO) as u32;
        let x = (tw - new_w) / 2;
        image::imageops::crop_imm(&trimmed, x, 0, new_w, th).to_image()
    } else if th as f32 > tw as f32 * MAX_RATIO {
        let new_h = (tw as f32 * MAX_RATIO) as u32;
        let y = (th - new_h) / 2;
        image::imageops::crop_imm(&trimmed, 0, y, tw, new_h).to_image()
    } else {
        trimmed
    };

    let (cw, ch) = trimmed.dimensions();
    let content_size = cw.max(ch);
    let margin = content_size / 32;
    let canvas_size = content_size + margin * 2;
    let mut canvas = image::RgbaImage::new(canvas_size, canvas_size);
    let x = ((canvas_size - cw) / 2) as i64;
    let y = ((canvas_size - ch) / 2) as i64;
    image::imageops::overlay(&mut canvas, &trimmed, x, y);
    canvas
}
