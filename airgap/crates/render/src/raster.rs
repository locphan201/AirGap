use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, Tree};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RasterError {
    #[error("SVG parsing error: {0}")]
    UsvgError(#[from] usvg::Error),

    #[error("Failed to allocate pixmap with dimensions {0}x{1}")]
    AllocationFailed(u32, u32),

    #[error("Invalid scale factor: {0}")]
    InvalidScale(f32),
}

/// A rasterized RGBA pixmap in premultiplied alpha format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramPixmap {
    pub width: u32,
    pub height: u32,
    /// Premultiplied RGBA bytes (tiny-skia native format).
    pub premultiplied_rgba: Vec<u8>,
}

/// Rasterizes an SVG string into a premultiplied RGBA pixmap at the specified device scale.
pub fn rasterize_svg_to_pixmap(svg_string: &str, scale: f32) -> Result<DiagramPixmap, RasterError> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(RasterError::InvalidScale(scale));
    }

    let options = usvg::Options {
        dpi: 96.0 * scale,
        ..Default::default()
    };

    let tree = Tree::from_str(svg_string, &options)?;

    let base_width = tree.size().width();
    let base_height = tree.size().height();

    let pixel_width = (base_width * scale).ceil() as u32;
    let pixel_height = (base_height * scale).ceil() as u32;

    let pixel_width = pixel_width.max(1);
    let pixel_height = pixel_height.max(1);

    let mut pixmap = Pixmap::new(pixel_width, pixel_height)
        .ok_or(RasterError::AllocationFailed(pixel_width, pixel_height))?;

    let transform = Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Ok(DiagramPixmap {
        width: pixel_width,
        height: pixel_height,
        premultiplied_rgba: pixmap.take(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rasterize_simple_svg() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50">
            <rect width="100" height="50" fill="red"/>
        </svg>"#;

        let pixmap = rasterize_svg_to_pixmap(svg, 1.0).unwrap();
        assert_eq!(pixmap.width, 100);
        assert_eq!(pixmap.height, 50);
        assert_eq!(pixmap.premultiplied_rgba.len(), 100 * 50 * 4);

        // HiDPI 2.0x scale
        let hidpi = rasterize_svg_to_pixmap(svg, 2.0).unwrap();
        assert_eq!(hidpi.width, 200);
        assert_eq!(hidpi.height, 100);
        assert_eq!(hidpi.premultiplied_rgba.len(), 200 * 100 * 4);
    }
}
