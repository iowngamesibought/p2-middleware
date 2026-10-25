fn qr_image(url: &str) -> Result<egui::ColorImage, ()> {
    let code = qrcode::QrCode::new(url.as_bytes()).map_err(|_| ())?;
    let width = code.width();
    let size = width + 8;
    let mut pixels = vec![Color32::WHITE; size * size];
    for y in 0..width {
        for x in 0..width {
            if code[(x, y)] == qrcode::Color::Dark {
                pixels[(y + 4) * size + x + 4] = Color32::BLACK;
            }
        }
    }
    Ok(egui::ColorImage::new([size, size], pixels))
}
