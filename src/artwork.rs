use std::path::PathBuf;

use image::{imageops::FilterType, DynamicImage, ImageReader, Limits};
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

const MAX_DIMENSION: u32 = 4_096;
const MAX_DECODE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Default)]
pub struct ArtworkCache {
    source: Option<String>,
    image: Option<DynamicImage>,
    rendered: Option<TerminalArtwork>,
}

impl ArtworkCache {
    pub fn update(&mut self, source: Option<&str>) {
        if self.source.as_deref() == source {
            return;
        }

        self.source = source.map(str::to_owned);
        self.image = source.and_then(load_local_artwork);
        self.rendered = None;
    }

    pub fn render_for(&mut self, width: u16, height: u16) -> Option<&TerminalArtwork> {
        let image = self.image.as_ref()?;
        let target = (width, height);
        if self
            .rendered
            .as_ref()
            .is_none_or(|artwork| artwork.target != target)
        {
            self.rendered = TerminalArtwork::from_image(image, width, height);
        }
        self.rendered.as_ref()
    }
}

pub struct TerminalArtwork {
    target: (u16, u16),
    width: u16,
    height: u16,
    cells: Vec<PixelPair>,
}

#[derive(Clone, Copy)]
struct PixelPair {
    top: Option<[u8; 3]>,
    bottom: Option<[u8; 3]>,
}

impl TerminalArtwork {
    fn from_image(image: &DynamicImage, width: u16, height: u16) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }

        let source_width = image.width().max(1);
        let source_height = image.height().max(1);
        let max_pixel_height = u32::from(height).saturating_mul(2);
        let scale = (f64::from(width) / f64::from(source_width))
            .min(f64::from(max_pixel_height) / f64::from(source_height));
        let pixel_width = (f64::from(source_width) * scale).round().max(1.0) as u32;
        let pixel_height = (f64::from(source_height) * scale).round().max(1.0) as u32;
        let resized = image
            .resize_exact(pixel_width, pixel_height, FilterType::Triangle)
            .to_rgba8();
        let cell_height = pixel_height.div_ceil(2) as u16;
        let mut cells = Vec::with_capacity((pixel_width as usize) * usize::from(cell_height));

        for cell_y in 0..u32::from(cell_height) {
            for x in 0..pixel_width {
                let top = visible_rgb(*resized.get_pixel(x, cell_y * 2));
                let bottom_y = cell_y * 2 + 1;
                let bottom = (bottom_y < pixel_height)
                    .then(|| visible_rgb(*resized.get_pixel(x, bottom_y)))
                    .flatten();
                cells.push(PixelPair { top, bottom });
            }
        }

        Some(Self {
            target: (width, height),
            width: pixel_width as u16,
            height: cell_height,
            cells,
        })
    }
}

impl Widget for &TerminalArtwork {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let start_x = area.x + area.width.saturating_sub(self.width) / 2;
        let start_y = area.y + area.height.saturating_sub(self.height) / 2;

        for y in 0..self.height.min(area.height) {
            for x in 0..self.width.min(area.width) {
                let pair = self.cells[usize::from(y) * usize::from(self.width) + usize::from(x)];
                let cell = &mut buffer[(start_x + x, start_y + y)];
                match (pair.top, pair.bottom) {
                    (Some(top), Some(bottom)) => {
                        cell.set_symbol("▀").set_fg(rgb(top)).set_bg(rgb(bottom));
                    }
                    (Some(top), None) => {
                        cell.set_symbol("▀").set_fg(rgb(top));
                    }
                    (None, Some(bottom)) => {
                        cell.set_symbol("▄").set_fg(rgb(bottom));
                    }
                    (None, None) => {}
                }
            }
        }
    }
}

fn visible_rgb(pixel: image::Rgba<u8>) -> Option<[u8; 3]> {
    (pixel[3] >= 16).then_some([pixel[0], pixel[1], pixel[2]])
}

fn rgb(value: [u8; 3]) -> Color {
    Color::Rgb(value[0], value[1], value[2])
}

fn load_local_artwork(source: &str) -> Option<DynamicImage> {
    let path = file_url_path(source)?;
    let mut reader = ImageReader::open(path).ok()?.with_guessed_format().ok()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    reader.decode().ok()
}

fn file_url_path(source: &str) -> Option<PathBuf> {
    let encoded = source.strip_prefix("file://")?;
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = hex_value(bytes[index + 1])?;
            let low = hex_value(bytes[index + 2])?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).ok().map(PathBuf::from)
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    #[test]
    fn decodes_file_url_paths() {
        assert_eq!(
            file_url_path("file:///tmp/album%20art.png").unwrap(),
            PathBuf::from("/tmp/album art.png")
        );
    }

    #[test]
    fn rejects_remote_and_malformed_urls() {
        assert!(file_url_path("https://example.com/cover.png").is_none());
        assert!(file_url_path("file:///tmp/cover%zz.png").is_none());
    }

    #[test]
    fn renders_two_pixels_per_terminal_cell() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(1, 2, |_, y| {
            if y == 0 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        }));
        let artwork = TerminalArtwork::from_image(&image, 1, 1).unwrap();
        let area = Rect::new(0, 0, 1, 1);
        let mut buffer = Buffer::empty(area);

        (&artwork).render(area, &mut buffer);

        assert_eq!(buffer[(0, 0)].symbol(), "▀");
        assert_eq!(buffer[(0, 0)].fg, Color::Rgb(255, 0, 0));
        assert_eq!(buffer[(0, 0)].bg, Color::Rgb(0, 0, 255));
    }
}
