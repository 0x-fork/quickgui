use std::{
    fmt,
    hash::{Hash, Hasher},
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use thiserror::Error;

use crate::{
    Rect, Size,
    animated_image::{AnimatedImage, ImageAsset},
};

/// Largest accepted width or height for a decoded image.
pub const MAX_IMAGE_DIMENSION: u32 = 4096;
/// Largest decoded RGBA allocation owned by one [`Image`].
pub const MAX_DECODED_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
/// Largest encoded file accepted by [`Image::open`].
pub const MAX_ENCODED_IMAGE_BYTES: u64 = 64 * 1024 * 1024;

static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_CUSTOM_RESOURCE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ImageId(u64);

/// Immutable, cheap-to-clone RGBA image data.
///
/// Clones share both identity and pixels. That identity lets every window reuse one GPU texture
/// until the renderer's bounded cache needs the space for a newer visible image.
#[derive(Clone)]
pub struct Image(Arc<ImageData>);

struct ImageData {
    id: ImageId,
    width: u32,
    height: u32,
    rgba: Arc<[u8]>,
}

impl Image {
    /// Construct an image from tightly packed, straight-alpha RGBA8 pixels.
    pub fn from_rgba(
        width: u32,
        height: u32,
        rgba: impl Into<Arc<[u8]>>,
    ) -> Result<Self, ImageError> {
        validate_dimensions(width, height)?;
        let expected = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(ImageError::TooLarge {
                bytes: u64::MAX,
                maximum: MAX_DECODED_IMAGE_BYTES,
            })?;
        if expected > MAX_DECODED_IMAGE_BYTES {
            return Err(ImageError::TooLarge {
                bytes: expected,
                maximum: MAX_DECODED_IMAGE_BYTES,
            });
        }
        let rgba = rgba.into();
        if rgba.len() as u64 != expected {
            return Err(ImageError::InvalidPixelLength {
                expected,
                actual: rgba.len(),
            });
        }
        let id = ImageId(NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed));
        Ok(Self(Arc::new(ImageData {
            id,
            width,
            height,
            rgba,
        })))
    }

    /// Decode PNG, JPEG, TIFF, WebP, or the first frame of a GIF from memory.
    ///
    /// Decoding is explicit and synchronous. Applications should perform it away from latency-
    /// sensitive input handling and then publish the resulting cheap-to-clone `Image`.
    pub fn decode(encoded: impl AsRef<[u8]>) -> Result<Self, ImageError> {
        let mut reader = image_codecs::ImageReader::new(Cursor::new(encoded.as_ref()))
            .with_guessed_format()
            .map_err(ImageError::Inspect)?;
        let mut limits = image_codecs::Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
        limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
        limits.max_alloc = Some(MAX_DECODED_IMAGE_BYTES);
        reader.limits(limits);
        let decoded = reader.decode().map_err(ImageError::Decode)?.into_rgba8();
        Self::from_rgba(decoded.width(), decoded.height(), decoded.into_raw())
    }

    /// Decode a supported image file synchronously with encoded and decoded size limits.
    ///
    /// Prefer passing a path to [`crate::img`] for event-driven background loading. This method is
    /// useful when an application already owns a background execution context.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path).map_err(|source| ImageError::Open {
            path: path.to_path_buf(),
            source,
        })?;
        if metadata.len() > MAX_ENCODED_IMAGE_BYTES {
            return Err(ImageError::EncodedTooLarge {
                bytes: metadata.len(),
                maximum: MAX_ENCODED_IMAGE_BYTES,
            });
        }
        let mut reader =
            image_codecs::ImageReader::open(path).map_err(|source| ImageError::Open {
                path: path.to_path_buf(),
                source,
            })?;
        let mut limits = image_codecs::Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
        limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
        limits.max_alloc = Some(MAX_DECODED_IMAGE_BYTES);
        reader.limits(limits);
        let decoded = reader.decode().map_err(ImageError::Decode)?.into_rgba8();
        Self::from_rgba(decoded.width(), decoded.height(), decoded.into_raw())
    }

    pub fn width(&self) -> u32 {
        self.0.width
    }

    pub fn height(&self) -> u32 {
        self.0.height
    }

    pub fn size(&self) -> Size {
        Size::new(self.width() as f32, self.height() as f32)
    }

    pub fn byte_len(&self) -> usize {
        self.0.rgba.len()
    }

    pub(crate) fn id(&self) -> ImageId {
        self.0.id
    }

    /// Tightly packed, straight-alpha RGBA8 pixels.
    pub fn rgba(&self) -> &[u8] {
        &self.0.rgba
    }
}

type CustomImageLoader = dyn Fn() -> Result<Image, Arc<str>> + Send + Sync;
type CustomAnimatedImageLoader = dyn Fn() -> Result<AnimatedImage, Arc<str>> + Send + Sync;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ImageResourceKey {
    Path(Arc<Path>),
    Asset { source: u64, path: Arc<str> },
    Custom(u64),
}

#[derive(Clone)]
enum ImageResourceLoader {
    Path(Arc<Path>),
    Asset {
        assets: crate::Assets,
        path: Arc<str>,
    },
    Custom(Arc<CustomImageLoader>),
    Animated(Arc<CustomAnimatedImageLoader>),
}

/// A cheap, stable handle to image content loaded away from the UI thread.
///
/// Path resources deduplicate by path. Custom resources deduplicate by handle identity, so retain
/// and clone a custom handle instead of constructing it inside every `View::render` call.
#[derive(Clone)]
pub struct ImageResource {
    key: ImageResourceKey,
    loader: ImageResourceLoader,
}

impl ImageResource {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path: Arc<Path> = Arc::from(path.into().into_boxed_path());
        Self {
            key: ImageResourceKey::Path(path.clone()),
            loader: ImageResourceLoader::Path(path),
        }
    }

    pub(crate) fn from_asset(assets: crate::Assets, path: Arc<str>) -> Self {
        Self {
            key: ImageResourceKey::Asset {
                source: assets.id(),
                path: path.clone(),
            },
            loader: ImageResourceLoader::Asset { assets, path },
        }
    }

    /// Create a retained custom background loader.
    pub fn custom<F, E>(loader: F) -> Self
    where
        F: Fn() -> Result<Image, E> + Send + Sync + 'static,
        E: fmt::Display,
    {
        let id = NEXT_CUSTOM_RESOURCE_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            key: ImageResourceKey::Custom(id),
            loader: ImageResourceLoader::Custom(Arc::new(move || {
                loader().map_err(|error| Arc::from(error.to_string()))
            })),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.loader {
            ImageResourceLoader::Path(path) => Some(path),
            ImageResourceLoader::Asset { .. }
            | ImageResourceLoader::Custom(_)
            | ImageResourceLoader::Animated(_) => None,
        }
    }

    pub(crate) fn key(&self) -> &ImageResourceKey {
        &self.key
    }

    pub(crate) fn load(&self) -> Result<ImageAsset, Arc<str>> {
        match &self.loader {
            ImageResourceLoader::Path(path) => {
                ImageAsset::open(path).map_err(|error| Arc::from(error.to_string()))
            }
            ImageResourceLoader::Asset { assets, path } => {
                let bytes = assets
                    .load_required(path)
                    .map_err(|error| Arc::from(error.to_string()))?;
                ImageAsset::decode(bytes.as_ref()).map_err(|error| Arc::from(error.to_string()))
            }
            ImageResourceLoader::Custom(loader) => loader().map(ImageAsset::Static),
            ImageResourceLoader::Animated(loader) => loader().map(ImageAsset::Animated),
        }
    }

    /// Create a retained custom loader for a decoded animation.
    pub fn custom_animated<F, E>(loader: F) -> Self
    where
        F: Fn() -> Result<AnimatedImage, E> + Send + Sync + 'static,
        E: fmt::Display,
    {
        let id = NEXT_CUSTOM_RESOURCE_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            key: ImageResourceKey::Custom(id),
            loader: ImageResourceLoader::Animated(Arc::new(move || {
                loader().map_err(|error| Arc::from(error.to_string()))
            })),
        }
    }
}

impl fmt::Debug for ImageResource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.key {
            ImageResourceKey::Path(path) => formatter
                .debug_tuple("ImageResource::Path")
                .field(path)
                .finish(),
            ImageResourceKey::Asset { source, path } => formatter
                .debug_struct("ImageResource::Asset")
                .field("source", source)
                .field("path", path)
                .finish(),
            ImageResourceKey::Custom(id) => formatter
                .debug_tuple("ImageResource::Custom")
                .field(id)
                .finish(),
        }
    }
}

impl PartialEq for ImageResource {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for ImageResource {}

impl Hash for ImageResource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl fmt::Debug for Image {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Image")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("bytes", &self.byte_len())
            .finish_non_exhaustive()
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for Image {}

/// A source accepted by [`crate::img`]. New resource kinds can be added without changing views.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ImageSource {
    Image(Image),
    Animated(AnimatedImage),
    Resource(ImageResource),
}

impl ImageSource {
    pub(crate) fn image(&self) -> Option<&Image> {
        match self {
            Self::Image(image) => Some(image),
            Self::Animated(_) | Self::Resource(_) => None,
        }
    }

    pub(crate) fn animated(&self) -> Option<&AnimatedImage> {
        match self {
            Self::Animated(animation) => Some(animation),
            Self::Image(_) | Self::Resource(_) => None,
        }
    }

    pub(crate) fn resource(&self) -> Option<&ImageResource> {
        match self {
            Self::Image(_) | Self::Animated(_) => None,
            Self::Resource(resource) => Some(resource),
        }
    }
}

impl From<Image> for ImageSource {
    fn from(image: Image) -> Self {
        Self::Image(image)
    }
}

impl From<&Image> for ImageSource {
    fn from(image: &Image) -> Self {
        Self::Image(image.clone())
    }
}

impl From<AnimatedImage> for ImageSource {
    fn from(animation: AnimatedImage) -> Self {
        Self::Animated(animation)
    }
}

impl From<&AnimatedImage> for ImageSource {
    fn from(animation: &AnimatedImage) -> Self {
        Self::Animated(animation.clone())
    }
}

impl From<ImageResource> for ImageSource {
    fn from(resource: ImageResource) -> Self {
        Self::Resource(resource)
    }
}

impl From<&ImageResource> for ImageSource {
    fn from(resource: &ImageResource) -> Self {
        Self::Resource(resource.clone())
    }
}

impl From<PathBuf> for ImageSource {
    fn from(path: PathBuf) -> Self {
        Self::Resource(ImageResource::from_path(path))
    }
}

impl From<&Path> for ImageSource {
    fn from(path: &Path) -> Self {
        Self::Resource(ImageResource::from_path(path))
    }
}

impl From<String> for ImageSource {
    fn from(path: String) -> Self {
        Self::Resource(ImageResource::from_path(path))
    }
}

impl From<&str> for ImageSource {
    fn from(path: &str) -> Self {
        Self::Resource(ImageResource::from_path(path))
    }
}

/// How image content is sized inside its layout box.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ObjectFit {
    Fill,
    #[default]
    Contain,
    Cover,
    ScaleDown,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ImageFit {
    pub destination: Rect,
    pub source_uv: Rect,
}

pub(crate) fn fit_image(bounds: Rect, intrinsic: Size, fit: ObjectFit) -> ImageFit {
    debug_assert!(!bounds.is_empty());
    debug_assert!(!intrinsic.is_empty());
    match fit {
        ObjectFit::Fill => ImageFit {
            destination: bounds,
            source_uv: Rect::new(0.0, 0.0, 1.0, 1.0),
        },
        ObjectFit::Contain => contain(bounds, intrinsic),
        ObjectFit::Cover => cover(bounds, intrinsic),
        ObjectFit::ScaleDown => {
            if intrinsic.width <= bounds.width && intrinsic.height <= bounds.height {
                unscaled(bounds, intrinsic)
            } else {
                contain(bounds, intrinsic)
            }
        }
        ObjectFit::None => unscaled(bounds, intrinsic),
    }
}

fn contain(bounds: Rect, intrinsic: Size) -> ImageFit {
    let scale = (bounds.width / intrinsic.width).min(bounds.height / intrinsic.height);
    let size = Size::new(intrinsic.width * scale, intrinsic.height * scale);
    ImageFit {
        destination: centered(bounds, size),
        source_uv: Rect::new(0.0, 0.0, 1.0, 1.0),
    }
}

fn cover(bounds: Rect, intrinsic: Size) -> ImageFit {
    let scale = (bounds.width / intrinsic.width).max(bounds.height / intrinsic.height);
    let visible_width = bounds.width / scale;
    let visible_height = bounds.height / scale;
    ImageFit {
        destination: bounds,
        source_uv: Rect::new(
            (intrinsic.width - visible_width) * 0.5 / intrinsic.width,
            (intrinsic.height - visible_height) * 0.5 / intrinsic.height,
            visible_width / intrinsic.width,
            visible_height / intrinsic.height,
        ),
    }
}

fn unscaled(bounds: Rect, intrinsic: Size) -> ImageFit {
    ImageFit {
        destination: centered(bounds, intrinsic),
        source_uv: Rect::new(0.0, 0.0, 1.0, 1.0),
    }
}

fn centered(bounds: Rect, size: Size) -> Rect {
    Rect::new(
        bounds.x + (bounds.width - size.width) * 0.5,
        bounds.y + (bounds.height - size.height) * 0.5,
        size.width,
        size.height,
    )
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), ImageError> {
    if width == 0 || height == 0 {
        return Err(ImageError::EmptyDimensions { width, height });
    }
    if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(ImageError::DimensionsTooLarge {
            width,
            height,
            maximum: MAX_IMAGE_DIMENSION,
        });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ImageError {
    #[error("image dimensions must be non-zero, got {width}x{height}")]
    EmptyDimensions { width: u32, height: u32 },
    #[error("image dimensions {width}x{height} exceed the {maximum}px per-axis limit")]
    DimensionsTooLarge {
        width: u32,
        height: u32,
        maximum: u32,
    },
    #[error("decoded image uses {bytes} bytes, exceeding the {maximum}-byte limit")]
    TooLarge { bytes: u64, maximum: u64 },
    #[error("encoded image uses {bytes} bytes, exceeding the {maximum}-byte limit")]
    EncodedTooLarge { bytes: u64, maximum: u64 },
    #[error("an animated image must contain at least one decodable frame")]
    EmptyAnimation,
    #[error("animation contains {frames} frames, exceeding the {maximum}-frame limit")]
    TooManyAnimationFrames { frames: usize, maximum: usize },
    #[error("decoded animation uses {bytes} bytes, exceeding the {maximum}-byte limit")]
    AnimationTooLarge { bytes: u64, maximum: u64 },
    #[error("the decoded image is static rather than animated")]
    NotAnimated,
    #[error("a finite animation must play at least one iteration")]
    ZeroAnimationIterations,
    #[error("RGBA data has {actual} bytes; exactly {expected} were required")]
    InvalidPixelLength { expected: u64, actual: usize },
    #[error("could not inspect encoded image data: {0}")]
    Inspect(#[source] std::io::Error),
    #[error("could not open image file {path}: {source}")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not decode image data: {0}")]
    Decode(#[source] image_codecs::ImageError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnimatedImageFrame, Assets, BundledAssets};
    use image_codecs::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    fn temporary_path(label: &str) -> PathBuf {
        static NEXT_FILE_ID: AtomicU64 = AtomicU64::new(1);
        std::env::temp_dir().join(format!(
            "quickgui-{label}-{}-{}.png",
            std::process::id(),
            NEXT_FILE_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn rgba_length_is_validated() {
        let error = Image::from_rgba(2, 2, vec![0; 15]).unwrap_err();
        assert!(matches!(
            error,
            ImageError::InvalidPixelLength {
                expected: 16,
                actual: 15
            }
        ));
    }

    #[test]
    fn cloned_images_keep_identity_without_copying_pixels() {
        let image = Image::from_rgba(1, 1, vec![255; 4]).unwrap();
        assert_eq!(image, image.clone());
    }

    #[test]
    fn open_decodes_a_supported_file_with_the_same_limits() {
        let path = temporary_path("open");
        let pixels = [7, 11, 13, 255];
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&pixels, 1, 1, ExtendedColorType::Rgba8)
            .unwrap();
        std::fs::write(&path, encoded).unwrap();

        let image = Image::open(&path).unwrap();
        let _ = std::fs::remove_file(path);
        assert_eq!(image.size(), Size::new(1.0, 1.0));
        assert_eq!(image.rgba(), pixels);
    }

    #[test]
    fn resource_clones_keep_custom_loader_identity_and_normalize_errors() {
        let resource = ImageResource::custom(|| Err::<Image, _>("not available"));
        assert_eq!(resource, resource.clone());
        assert_eq!(resource.load().unwrap_err().as_ref(), "not available");
    }

    #[test]
    fn asset_resources_deduplicate_by_source_and_path_and_decode_on_demand() {
        let pixels = [7, 11, 13, 255];
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&pixels, 1, 1, ExtendedColorType::Rgba8)
            .unwrap();

        let mut bundle = BundledAssets::new();
        bundle.insert("images/pixel.png", encoded).unwrap();
        let assets = Assets::new(bundle.clone());
        let first = assets.image("images/pixel.png").unwrap();
        let second = assets.image("images/pixel.png").unwrap();
        assert_eq!(first, second);

        let ImageAsset::Static(decoded) = first.load().unwrap() else {
            panic!("PNG asset decoded as an animation");
        };
        assert_eq!(decoded.size(), Size::new(1.0, 1.0));
        assert_eq!(decoded.rgba(), pixels);

        let other_source = Assets::new(bundle);
        assert_ne!(
            first,
            other_source.image("images/pixel.png").unwrap(),
            "distinct application asset sources must not alias renderer cache entries"
        );
    }

    #[test]
    fn custom_animated_resources_preserve_the_decoded_asset() {
        let animation = AnimatedImage::new([
            AnimatedImageFrame::new(
                Image::from_rgba(1, 1, vec![1, 2, 3, 255]).unwrap(),
                std::time::Duration::from_millis(40),
            ),
            AnimatedImageFrame::new(
                Image::from_rgba(1, 1, vec![4, 5, 6, 255]).unwrap(),
                std::time::Duration::from_millis(60),
            ),
        ])
        .unwrap();
        let expected = animation.clone();
        let resource =
            ImageResource::custom_animated(move || Ok::<_, &'static str>(animation.clone()));
        assert_eq!(resource.load().unwrap(), ImageAsset::Animated(expected));
    }

    #[test]
    fn contain_centers_the_complete_source() {
        let fitted = fit_image(
            Rect::new(10.0, 20.0, 200.0, 200.0),
            Size::new(400.0, 200.0),
            ObjectFit::Contain,
        );
        assert_eq!(fitted.destination, Rect::new(10.0, 70.0, 200.0, 100.0));
        assert_eq!(fitted.source_uv, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn cover_centers_a_normalized_source_crop() {
        let fitted = fit_image(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Size::new(200.0, 100.0),
            ObjectFit::Cover,
        );
        assert_eq!(fitted.destination, Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(fitted.source_uv, Rect::new(0.25, 0.0, 0.5, 1.0));
    }

    #[test]
    fn scale_down_never_enlarges_small_images() {
        let fitted = fit_image(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Size::new(20.0, 40.0),
            ObjectFit::ScaleDown,
        );
        assert_eq!(fitted.destination, Rect::new(40.0, 30.0, 20.0, 40.0));
    }
}
