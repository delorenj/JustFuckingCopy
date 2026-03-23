use ab_glyph::{FontRef, PxScale};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_circle_mut, draw_text_mut, text_size};
use tauri::image::Image as TauriImage;
use tauri::{AppHandle, Manager};

const BASE_ICON: &[u8] = include_bytes!("../icons/icon.png");
const FONT_BYTES: &[u8] = include_bytes!("fonts/DejaVuSans-Bold.ttf");

/// Update the tray icon to show a red badge with `count`.
/// When count is 0, reset to the plain icon.
pub fn update_tray_icon(app: &AppHandle, count: usize) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };

    if count == 0 {
        let icon = tauri::include_image!("./icons/icon.png");
        let _ = tray.set_icon(Some(icon));
        let _ = tray.set_icon_as_template(true);
        return;
    }

    match render_badged_icon(count) {
        Ok((rgba, width, height)) => {
            let icon = TauriImage::new_owned(rgba, width, height);
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_icon_as_template(false);
        }
        Err(e) => {
            eprintln!("[JFC tray_badge] Failed to render badge: {e}");
        }
    }
}

/// Renders the base icon on a larger canvas with a badge circle overlapping
/// the bottom-right corner, extending past the icon boundary.
/// Returns (rgba_bytes, width, height).
fn render_badged_icon(count: usize) -> Result<(Vec<u8>, u32, u32), String> {
    let base_img = image::load_from_memory(BASE_ICON)
        .map_err(|e| format!("Failed to decode base icon: {e}"))?
        .into_rgba8();

    let icon_size = base_img.width(); // 128

    // Badge radius: 60% of icon size so it dominates at small tray sizes
    let badge_r = (icon_size as f64 * 0.60) as i32; // ~77px on 128px icon
    let border_w = 4i32;

    // Canvas must be large enough to hold icon + badge overflow.
    // Badge center sits on the icon's bottom-right corner.
    // Badge extends badge_r pixels past that corner in each direction.
    let canvas_size = icon_size + badge_r as u32 + border_w as u32;
    let mut canvas = RgbaImage::new(canvas_size, canvas_size);

    // Copy base icon onto canvas at (0, 0)
    for y in 0..base_img.height() {
        for x in 0..base_img.width() {
            canvas.put_pixel(x, y, *base_img.get_pixel(x, y));
        }
    }

    // Badge center: on the bottom-right corner of the icon
    let cx = icon_size as i32;
    let cy = icon_size as i32;

    // White border circle, then red fill on top
    draw_filled_circle_mut(
        &mut canvas,
        (cx, cy),
        badge_r + border_w,
        Rgba([255, 255, 255, 255]),
    );
    draw_filled_circle_mut(&mut canvas, (cx, cy), badge_r, Rgba([220, 38, 38, 255]));

    // Render text
    let label = if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    };

    let font = FontRef::try_from_slice(FONT_BYTES)
        .map_err(|e| format!("Failed to parse font: {e}"))?;

    // Size text to fill ~80% of badge diameter
    let max_text_w = (badge_r as f64 * 1.6) as u32;
    let mut font_size = badge_r as f32 * 1.4;
    let (mut tw, mut th) = text_size(PxScale::from(font_size), &font, &label);
    while tw > max_text_w && font_size > 10.0 {
        font_size -= 2.0;
        let dims = text_size(PxScale::from(font_size), &font, &label);
        tw = dims.0;
        th = dims.1;
    }

    // Center text on badge
    let text_x = cx - tw as i32 / 2;
    let text_y = cy - th as i32 / 2;

    draw_text_mut(
        &mut canvas,
        Rgba([255, 255, 255, 255]),
        text_x,
        text_y,
        PxScale::from(font_size),
        &font,
        &label,
    );

    let (out_w, out_h) = (canvas.width(), canvas.height());
    Ok((canvas.into_raw(), out_w, out_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_is_larger_than_base_icon() {
        let (_, w, h) = render_badged_icon(1).expect("should render");
        // Canvas must be bigger than 128x128 to allow badge overflow
        assert!(w > 128, "Canvas width {w} should exceed icon size");
        assert!(h > 128, "Canvas height {h} should exceed icon size");
        assert_eq!(w, h, "Canvas should be square");
    }

    #[test]
    fn render_single_digit_badge() {
        let (rgba, w, h) = render_badged_icon(3).expect("should render badge for count=3");
        assert_eq!(rgba.len(), (w * h * 4) as usize);
        let img = RgbaImage::from_raw(w, h, rgba).unwrap();
        // Red pixels should exist in the bottom-right area (badge location)
        let has_red = (h / 2..h).any(|y| {
            (w / 2..w).any(|x| {
                let p = img.get_pixel(x, y);
                p[0] > 200 && p[1] < 60 && p[2] < 60
            })
        });
        assert!(has_red, "Badge should contain red pixels in bottom-right");
    }

    #[test]
    fn render_double_digit_badge() {
        let (rgba, w, h) = render_badged_icon(42).expect("should render badge for count=42");
        assert_eq!(rgba.len(), (w * h * 4) as usize);
    }

    #[test]
    fn render_overflow_badge() {
        let (rgba, w, h) = render_badged_icon(150).expect("should render badge for count=150");
        assert_eq!(rgba.len(), (w * h * 4) as usize);
    }

    #[test]
    fn badge_has_white_text() {
        let (rgba, w, h) = render_badged_icon(5).expect("should render badge for count=5");
        let img = RgbaImage::from_raw(w, h, rgba).unwrap();
        // White text pixels in the badge area (bottom-right quadrant, near icon corner)
        let badge_zone_y = (h * 3 / 8)..h;
        let badge_zone_x = (w * 3 / 8)..w;
        let has_white = badge_zone_y.into_iter().any(|y| {
            badge_zone_x.clone().any(|x| {
                let p = img.get_pixel(x, y);
                p[0] > 240 && p[1] > 240 && p[2] > 240 && p[3] > 200
            })
        });
        assert!(has_white, "Badge should contain white text pixels");
    }
}
