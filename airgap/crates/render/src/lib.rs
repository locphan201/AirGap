pub mod cache;
pub mod fallback;
pub mod highlight;
pub mod image;
pub mod markdown;
pub mod mermaid;
pub mod model;
pub mod raster;

pub use cache::{DiagramCache, DiagramCacheKey};
pub use fallback::DiagramErrorFallback;
pub use highlight::{CodeThemePalette, HighlightedSpan, SyntaxHighlighter};
pub use image::{resolve_local_image_path, ImageResolutionError};
pub use markdown::{generate_slug, parse_markdown, LineIndex};
pub use mermaid::{detect_diagram_type, render_mermaid_to_svg, DiagramType, MermaidError};
pub use model::{
    Block, CalloutKind, ColumnAlignment, DocModel, InlineSpan, InlineStyle, ListItem, SourceSpan,
};
pub use raster::{rasterize_svg_to_pixmap, DiagramPixmap, RasterError};
