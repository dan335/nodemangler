//! The value system for the node graph engine.
//!
//! [`Value`] is the universal data type carried on every node input and output.
//! It supports 14 variant types (booleans, numbers, strings, colors, images, etc.)
//! along with type conversion, fingerprinting for cache invalidation, and thumbnail
//! generation for UI previews.
//!
//! [`ValueType`] is the type-level discriminant used for connection validation and
//! conversion tables without carrying actual data.

use std::path::PathBuf;
use std::sync::Arc;

use image::{imageops::FilterType, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::{
    color::Color, float_image::FloatImage, get_id,
    operations::images::noise::worley_distance::NoiseWorleyDistanceFunction,
    thumbnail::Thumbnail,
};

/// Dimensions (width, height) used when generating thumbnail previews.
pub const THUMBNAIL_SIZE: [u32; 2] = [150, 150];

/// The universal data type carried on every node input and output.
///
/// Values flow through the graph along connections, are converted between
/// compatible types automatically, and can generate thumbnails for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    /// A boolean true/false value.
    Bool(bool),
    /// A 32-bit signed integer.
    Integer(i32),
    /// A 32-bit floating-point number.
    Decimal(f32),
    /// A UTF-8 text string.
    #[serde(alias = "String")]
    Text(String),
    /// An sRGBA color with floating-point channels.
    Color(Color),
    /// A full raster image wrapped in an `Arc` for cheap cloning.
    ///
    /// Uses [`FloatImage`] internally: 1–4 channel f32 buffer.
    /// Format conversion (Rgb8, Gray16, etc.) only happens at I/O boundaries.
    #[serde(alias = "DynamicImage")]
    Image {
        /// The image data, reference-counted for efficient sharing.
        #[serde(with = "crate::float_image_serde")]
        data: Arc<FloatImage>,
        /// A unique ID regenerated each time the image changes, used for cache invalidation.
        change_id: String,
    },
    /// A filesystem path (file or directory).
    Path(PathBuf),
    /// An image resampling filter algorithm (e.g. Lanczos3, Nearest).
    #[serde(
        serialize_with = "serialize_filter_type",
        deserialize_with = "deserialize_filter_type"
    )]
    FilterType(FilterType),
    /// A pixel color format (e.g. Rgba8, Rgb16).
    ColorFormat(ColorFormat),
    /// An image file format (e.g. PNG, JPEG).
    #[serde(
        serialize_with = "serialize_image_format",
        deserialize_with = "deserialize_image_format"
    )]
    ImageType(image::ImageFormat),
    /// A trigger signal that forces downstream re-evaluation without carrying data.
    Trigger,
    /// Distance function variant for Worley noise generation.
    NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction),
    /// A color space identifier for color space conversion operations.
    ColorSpace(crate::color::color_spaces::ColorSpace),
    /// A blend mode for image/color compositing operations.
    BlendMode(crate::color::blend::BlendMode),
    /// Horizontal alignment for text rendering.
    TextHAlign(TextHAlign),
    /// Vertical alignment for text rendering.
    TextVAlign(TextVAlign),
    /// The wrapper file format of a video (MP4, MKV, WebM, ...).
    VideoContainer(VideoContainer),
    /// The video stream codec (H.264, VP9, AV1, ...).
    VideoCodec(VideoCodec),
    /// A lightweight handle to a video file — carries path + metadata.
    /// Produced by the `video from file` node; consumed by extract-frame ops.
    Video(VideoRef),
}

/// Modes for file/folder picker dialogs.
pub enum PathType {
    /// Pick a single file.
    PickFile,
    /// Pick multiple files.
    PickFiles,
    /// Pick a single folder.
    PickFolder,
    /// Pick multiple folders.
    PickFolders,
    /// Save to a file path.
    SaveFile,
}

impl Default for Value {
    fn default() -> Self {
        Value::Bool(false)
    }
}

impl Value {
    /// Generate a thumbnail preview for this value, suitable for display in the UI.
    ///
    /// Colors produce a solid-fill image swatch. Images produce a downscaled thumbnail.
    /// Scalar and enum types produce a text representation.
    pub fn create_thumbnail(&self) -> Option<Thumbnail> {
        match &self {
            Value::Color(color) => {
                let rgb = color.to_srgb_u8();
                let color = image::Rgba([rgb.0, rgb.1, rgb.2, rgb.3]);
                let mut img = RgbaImage::new(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]);
                for x in 0..THUMBNAIL_SIZE[0] {
                    for y in 0..THUMBNAIL_SIZE[1] {
                        img.put_pixel(x, y, color);
                    }
                }

                Some(Thumbnail::Image(img))
            }
            Value::Image { data, change_id: _ } => Some(Thumbnail::Image(
                data.resize_fit(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]).to_rgba8(),
            )),
            Value::Bool(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Integer(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Decimal(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::Text(value) => Some(Thumbnail::Text(value.clone())),
            Value::Path(path) => Some(Thumbnail::Text(path.to_str().unwrap_or("none").to_string())),
            Value::FilterType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::ColorFormat(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::Trigger => Some(Thumbnail::Text("trigger".to_string())),
            Value::ImageType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::NoiseWorleyDistanceFunction(value) => {
                Some(Thumbnail::Text(format!("{:?}", value)))
            }
            Value::ColorSpace(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::BlendMode(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::TextHAlign(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::TextVAlign(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::VideoContainer(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::VideoCodec(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            // Thumbnail is deferred to the async ThumbnailService, which
            // resolves the first frame from the decoder cache off-thread.
            // Returning None here signals the engine's emit sites to
            // enqueue instead of inlining.
            Value::Video(_) => None,
        }
    }

    /// Zero-allocation fingerprint for cache comparison.
    /// Returns a u64 hash that changes when the value changes.
    pub fn fingerprint(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        std::mem::discriminant(self).hash(&mut h);
        match self {
            Value::Bool(v) => v.hash(&mut h),
            Value::Integer(v) => v.hash(&mut h),
            Value::Decimal(v) => v.to_bits().hash(&mut h),
            Value::Text(v) => v.hash(&mut h),
            Value::Color(c) => {
                c.r.to_bits().hash(&mut h);
                c.g.to_bits().hash(&mut h);
                c.b.to_bits().hash(&mut h);
                c.a.to_bits().hash(&mut h);
            }
            Value::Image { data: _, change_id } => change_id.hash(&mut h),
            Value::Path(p) => p.hash(&mut h),
            Value::FilterType(f) => (*f as u8).hash(&mut h),
            Value::ColorFormat(cf) => (*cf as u8).hash(&mut h),
            Value::ImageType(it) => format!("{:?}", it).hash(&mut h),
            Value::Trigger => 0u8.hash(&mut h), // always same — triggers re-run via is_dirty
            Value::NoiseWorleyDistanceFunction(w) => format!("{:?}", w).hash(&mut h),
            Value::ColorSpace(cs) => format!("{:?}", cs).hash(&mut h),
            Value::BlendMode(bm) => format!("{:?}", bm).hash(&mut h),
            Value::TextHAlign(v) => format!("{:?}", v).hash(&mut h),
            Value::TextVAlign(v) => format!("{:?}", v).hash(&mut h),
            Value::VideoContainer(v) => (*v as u8).hash(&mut h),
            Value::VideoCodec(v) => (*v as u8).hash(&mut h),
            Value::Video(v) => v.path.hash(&mut h),
        }
        h.finish()
    }

    /// Return the type-level discriminant for this value, used in connection
    /// validation and conversion lookups.
    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Bool(_) => ValueType::Bool,
            Value::Integer(_) => ValueType::Integer,
            Value::Decimal(_) => ValueType::Decimal,
            Value::Text(_) => ValueType::Text,
            Value::Color(_) => ValueType::Color,
            Value::ColorFormat(_) => ValueType::ColorFormat,
            Value::Trigger => ValueType::Trigger,
            Value::FilterType(_) => ValueType::FilterType,
            Value::Path(_) => ValueType::Path,
            Value::Image {
                data: _,
                change_id: _,
            } => ValueType::Image,
            Value::ImageType(_) => ValueType::ImageType,
            Value::NoiseWorleyDistanceFunction(_) => ValueType::NoiseWorleyDistanceFunction,
            Value::ColorSpace(_) => ValueType::ColorSpace,
            Value::BlendMode(_) => ValueType::BlendMode,
            Value::TextHAlign(_) => ValueType::TextHAlign,
            Value::TextVAlign(_) => ValueType::TextVAlign,
            Value::VideoContainer(_) => ValueType::VideoContainer,
            Value::VideoCodec(_) => ValueType::VideoCodec,
            Value::Video(_) => ValueType::Video,
        }
    }

    /// Attempt to convert this value to the target type.
    ///
    /// Conversions follow the rules defined by [`ValueType::valid_conversions`].
    /// Numeric types convert freely between each other. Scalars can convert to
    /// colors (grayscale) and 1x1 images. String parsing is attempted for
    /// string-to-numeric conversions. Returns a [`ConversionError`] if the
    /// conversion is not supported or fails at runtime.
    pub fn try_convert_to(&self, other: ValueType) -> Result<Value, ConversionError> {
        match self {
            Value::Bool(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a)),
                ValueType::Integer => {
                    if *a {
                        Ok(Value::Integer(1))
                    } else {
                        Ok(Value::Integer(0))
                    }
                }
                ValueType::Decimal => {
                    if *a {
                        Ok(Value::Decimal(1.0))
                    } else {
                        Ok(Value::Decimal(0.0))
                    }
                }
                ValueType::Text => Ok(Value::Text(a.to_string())),
                ValueType::Color => {
                    if *a {
                        Ok(Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)))
                    } else {
                        Ok(Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)))
                    }
                }
                ValueType::Image => {
                    let v = if *a { 1.0 } else { 0.0 };
                    Ok(Value::Image {
                        data: Arc::new(FloatImage::from_pixel(1, 1, 1, &[v])),
                        change_id: get_id(),
                    })
                }
                _ => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
            },
            Value::Integer(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a != 0)),
                ValueType::Integer => Ok(Value::Integer(*a)),
                ValueType::Decimal => Ok(Value::Decimal(*a as f32)),
                ValueType::Text => Ok(Value::Text(a.to_string())),
                ValueType::Color => {
                    let v = (*a).clamp(0, 255) as f32 / 255.0;
                    Ok(Value::Color(Color::from_srgb_float(v, v, v, 1.0)))
                }
                ValueType::Image => {
                    let v = (*a).clamp(0, 255) as f32 / 255.0;
                    Ok(Value::Image {
                        data: Arc::new(FloatImage::from_pixel(1, 1, 1, &[v])),
                        change_id: get_id(),
                    })
                }
                _ => Err(ConversionError {
                    message: "Unable to convert integer to this type.".to_string(),
                }),
            },
            Value::Decimal(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a != 0.0)),
                ValueType::Integer => Ok(Value::Integer(*a as i32)),
                ValueType::Decimal => Ok(Value::Decimal(*a)),
                ValueType::Text => Ok(Value::Text(a.to_string())),
                ValueType::Color => {
                    let v = a.clamp(0.0, 1.0);
                    Ok(Value::Color(Color::from_srgb_float(v, v, v, 1.0)))
                }
                ValueType::Image => {
                    let v = a.clamp(0.0, 1.0);
                    Ok(Value::Image {
                        data: Arc::new(FloatImage::from_pixel(1, 1, 1, &[v])),
                        change_id: get_id(),
                    })
                }
                _ => Err(ConversionError {
                    message: "Unable to convert decimal to this type.".to_string(),
                }),
            },
            Value::Color(a) => match other {
                ValueType::Bool => Ok(Value::Bool(a.r != 0.0 || a.g != 0.0 || a.b != 0.0)),
                ValueType::Integer => {
                    let lum = 0.2126 * a.r + 0.7152 * a.g + 0.0722 * a.b;
                    Ok(Value::Integer((lum.clamp(0.0, 1.0) * 255.0) as i32))
                }
                ValueType::Decimal => {
                    let lum = 0.2126 * a.r + 0.7152 * a.g + 0.0722 * a.b;
                    Ok(Value::Decimal(lum))
                }
                ValueType::Text => Ok(Value::Text(format!("rgba({}, {}, {}, {})", a.r, a.g, a.b, a.a))),
                ValueType::Color => Ok(Value::Color(*a)),
                ValueType::Image => {
                    let srgb = a.to_srgb_float();
                    Ok(Value::Image {
                        data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[srgb.0, srgb.1, srgb.2, srgb.3])),
                        change_id: get_id(),
                    })
                }
                _ => Err(ConversionError {
                    message: "Unable to convert color to this type.".to_string(),
                }),
            },
            Value::FilterType(a) => match other {
                ValueType::FilterType => Ok(Value::FilterType(*a)),
                _ => Err(ConversionError {
                    message: "Unable to convert filter type to bool.".to_string(),
                }),
            },
            Value::ColorFormat(a) => match other {
                ValueType::ColorFormat => Ok(Value::ColorFormat(*a)),
                _ => Err(ConversionError {
                    message: "Unable to convert image type to bool.".to_string(),
                }),
            },
            Value::Trigger => todo!(),
            Value::Image { data, change_id } => match other {
                ValueType::Image => Ok(Value::Image {
                    data: data.clone(),
                    change_id: change_id.clone(),
                }),
                _ => Err(ConversionError {
                    message: "Unable to convert image to this type.".to_string(),
                }),
            },
            Value::Path(path) => match other {
                ValueType::Text => {
                    if let Ok(path_string) = path.clone().into_os_string().into_string() {
                        Ok(Value::Text(path_string))
                    } else {
                        Err(ConversionError {
                            message: "Unable to convert integer to image format.".to_string(),
                        })
                    }
                }
                ValueType::Path => Ok(Value::Path(path.clone())),
                _ => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            },
            Value::ImageType(image_format) => match other {
                ValueType::ImageType => Ok(Value::ImageType(*image_format)),
                _ => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
            },
            Value::NoiseWorleyDistanceFunction(a) => match other {
                ValueType::NoiseWorleyDistanceFunction => {
                    Ok(Value::NoiseWorleyDistanceFunction(a.clone()))
                }
                _ => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
            },
            Value::ColorSpace(a) => match other {
                ValueType::ColorSpace => Ok(Value::ColorSpace(*a)),
                _ => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
            },
            Value::BlendMode(a) => match other {
                ValueType::BlendMode => Ok(Value::BlendMode(a.clone())),
                _ => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
            },
            Value::TextHAlign(a) => match other {
                ValueType::TextHAlign => Ok(Value::TextHAlign(*a)),
                _ => Err(ConversionError { message: "Unable to convert.".to_string() }),
            },
            Value::TextVAlign(a) => match other {
                ValueType::TextVAlign => Ok(Value::TextVAlign(*a)),
                _ => Err(ConversionError { message: "Unable to convert.".to_string() }),
            },
            Value::VideoContainer(a) => match other {
                ValueType::VideoContainer => Ok(Value::VideoContainer(*a)),
                _ => Err(ConversionError { message: "Unable to convert.".to_string() }),
            },
            Value::VideoCodec(a) => match other {
                ValueType::VideoCodec => Ok(Value::VideoCodec(*a)),
                _ => Err(ConversionError { message: "Unable to convert.".to_string() }),
            },
            Value::Video(a) => match other {
                ValueType::Video => Ok(Value::Video(a.clone())),
                _ => Err(ConversionError { message: "Unable to convert.".to_string() }),
            },
            Value::Text(a) => match other {
                ValueType::Text => Ok(Value::Text(a.clone())),
                ValueType::Path => Ok(Value::Path(PathBuf::from(a))),
                ValueType::Bool => {
                    let result: Result<bool, _> = a.parse();
                    match result {
                        Ok(r) => Ok(Value::Bool(r)),
                        Err(_) => Err(ConversionError { message: "Error converting text to bool.".to_string() }),
                    }
                }
                ValueType::Integer => {
                    let result: Result<i32, _> = a.parse();
                    match result {
                        Ok(r) => Ok(Value::Integer(r)),
                        Err(_) => Err(ConversionError { message: "Error converting text to integer.".to_string() }),
                    }
                }
                ValueType::Decimal => {
                    let result: Result<f32, _> = a.parse();
                    match result {
                        Ok(r) => Ok(Value::Decimal(r)),
                        Err(_) => Err(ConversionError { message: "Error converting text to decimal.".to_string() }),
                    }
                }
                ValueType::NoiseWorleyDistanceFunction => {
                    match a.to_lowercase().as_str() {
                        "chebyshev" => Ok(Value::NoiseWorleyDistanceFunction(crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::Chebyshev)),
                        "euclidean" => Ok(Value::NoiseWorleyDistanceFunction(crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::Euclidean)),
                        "euclideansquared" | "euclidean_squared" | "euclidean squared" => Ok(Value::NoiseWorleyDistanceFunction(crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::EuclideanSquared)),
                        "manhattan" => Ok(Value::NoiseWorleyDistanceFunction(crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::Manhattan)),
                        "quadratic" => Ok(Value::NoiseWorleyDistanceFunction(crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::Quadratic)),
                        _ => Err(ConversionError { message: format!("Unknown distance function '{}'. Expected: chebyshev, euclidean, euclidean_squared, manhattan, quadratic.", a) }),
                    }
                }
                _ => Err(ConversionError { message: "Unable to convert text to this type.".to_string() }),
            },
        }
    }
}

/// Type-level discriminant for [`Value`], used for connection validation and
/// conversion tables without carrying actual data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValueType {
    /// Boolean type.
    Bool,
    /// 32-bit signed integer type.
    Integer,
    /// 32-bit float type.
    Decimal,
    /// UTF-8 text string type.
    #[serde(alias = "String")]
    Text,
    /// sRGBA color type.
    Color,
    /// Image resampling filter type.
    FilterType,
    /// Pixel color format type.
    ColorFormat,
    /// Image file format type.
    ImageType,
    /// Trigger signal type (carries no data).
    Trigger,
    /// Raster image type (FloatImage: 1–4 channel f32).
    #[serde(alias = "DynamicImage")]
    Image,
    /// Filesystem path type.
    Path,
    /// Worley noise distance function type.
    NoiseWorleyDistanceFunction,
    /// Color space identifier type.
    ColorSpace,
    /// Blend mode type.
    BlendMode,
    /// Horizontal text alignment type.
    TextHAlign,
    /// Vertical text alignment type.
    TextVAlign,
    /// Video container/wrapper format type (MP4, MKV, WebM, ...).
    VideoContainer,
    /// Video stream codec type (H.264, VP9, AV1, ...).
    VideoCodec,
    /// A video-file handle ([`VideoRef`] = path + metadata).
    Video,
}

impl ValueType {
    /// Return the standard set of value types available for general use.
    pub fn types() -> [ValueType; 10] {
        let types: [ValueType; 10] = [
            ValueType::Bool,
            ValueType::Integer,
            ValueType::Decimal,
            ValueType::Text,
            ValueType::Color,
            ValueType::FilterType,
            ValueType::ColorFormat,
            ValueType::Trigger,
            ValueType::Image,
            ValueType::Path,
        ];

        types
    }

    /// Return a default `Value` for this type, used when adapting pass-through inputs.
    pub fn default_value(&self) -> Value {
        match self {
            ValueType::Bool => Value::Bool(false),
            ValueType::Integer => Value::Integer(0),
            ValueType::Decimal => Value::Decimal(0.0),
            ValueType::Text => Value::Text(String::new()),
            ValueType::Color => Value::Color(Color::default()),
            ValueType::FilterType => Value::FilterType(FilterType::Nearest),
            ValueType::ColorFormat => Value::ColorFormat(crate::value::ColorFormat::Rgba32F),
            ValueType::ImageType => Value::ImageType(image::ImageFormat::Png),
            ValueType::Trigger => Value::Trigger,
            ValueType::Image => Value::Image {
                data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[1.0, 1.0, 1.0, 1.0])),
                change_id: get_id(),
            },
            ValueType::Path => Value::Path(PathBuf::new()),
            ValueType::NoiseWorleyDistanceFunction => {
                Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::Euclidean)
            }
            ValueType::ColorSpace => Value::ColorSpace(crate::color::color_spaces::ColorSpace::Srgb),
            ValueType::BlendMode => Value::BlendMode(crate::color::blend::BlendMode::Over),
            ValueType::TextHAlign => Value::TextHAlign(TextHAlign::Center),
            ValueType::TextVAlign => Value::TextVAlign(TextVAlign::Middle),
            ValueType::VideoContainer => Value::VideoContainer(VideoContainer::Mp4),
            ValueType::VideoCodec => Value::VideoCodec(VideoCodec::H264),
            ValueType::Video => Value::Video(VideoRef::default()),
        }
    }

    /// Return a human-readable name for this type, used in the UI.
    pub fn value_name(&self) -> String {
        match self {
            ValueType::Bool => "bool".to_string(),
            ValueType::Integer => "integer".to_string(),
            ValueType::Decimal => "decimal".to_string(),
            ValueType::Text => "text".to_string(),
            ValueType::Color => "color".to_string(),
            ValueType::FilterType => "filter type".to_string(),
            ValueType::ColorFormat => "color format".to_string(),
            ValueType::Trigger => "trigger".to_string(),
            ValueType::Image => "image".to_string(),
            ValueType::Path => "path".to_string(),
            ValueType::ImageType => "image format".to_string(),
            ValueType::NoiseWorleyDistanceFunction => "worley noise distance function".to_string(),
            ValueType::ColorSpace => "color space".to_string(),
            ValueType::BlendMode => "blend mode".to_string(),
            ValueType::TextHAlign => "text h-align".to_string(),
            ValueType::TextVAlign => "text v-align".to_string(),
            ValueType::VideoContainer => "video container".to_string(),
            ValueType::VideoCodec => "video codec".to_string(),
            ValueType::Video => "video".to_string(),
        }
    }

    /// Return the file extensions that can be opened for each value type.
    /// Currently only `DynamicImage` has associated file extensions.
    pub fn file_extensions(value_type: &ValueType) -> Vec<String> {
        match value_type {
            ValueType::Image => {
                let mut list = vec![];

                for image_format in ImageType::types().iter() {
                    let ext = image_format.format().extensions_str()[0];
                    list.push(ext.to_string());
                }

                list
            }
            _ => vec![],
        }
    }

    /// Return the list of types that this type can be converted **to**.
    ///
    /// This defines the connection compatibility rules: an output of this type
    /// can connect to any input whose type appears in the returned list.
    pub fn valid_conversions(&self) -> Vec<ValueType> {
        match self {
            ValueType::Bool => vec![
                ValueType::Bool,
                ValueType::Integer,
                ValueType::Decimal,
                ValueType::Text,
                ValueType::Color,
                ValueType::Image,
                ValueType::Trigger,
            ],
            ValueType::Integer => vec![
                ValueType::Bool,
                ValueType::Integer,
                ValueType::Decimal,
                ValueType::Text,
                ValueType::Color,
                ValueType::Image,
                ValueType::Trigger,
            ],
            ValueType::Decimal => vec![
                ValueType::Bool,
                ValueType::Integer,
                ValueType::Decimal,
                ValueType::Text,
                ValueType::Color,
                ValueType::Image,
                ValueType::Trigger,
            ],
            ValueType::Text => vec![ValueType::Text, ValueType::Path, ValueType::Trigger],
            ValueType::Color => vec![
                ValueType::Bool,
                ValueType::Integer,
                ValueType::Decimal,
                ValueType::Text,
                ValueType::Color,
                ValueType::Image,
                ValueType::Trigger,
            ],
            ValueType::Image => vec![ValueType::Image, ValueType::Trigger],
            ValueType::Path => vec![ValueType::Text, ValueType::Path, ValueType::Trigger],
            ValueType::FilterType => {
                vec![ValueType::FilterType, ValueType::Trigger]
            }
            ValueType::ColorFormat => vec![
                ValueType::ColorFormat,
                ValueType::Trigger,
            ],
            ValueType::Trigger => vec![ValueType::Trigger],
            ValueType::ImageType => vec![ValueType::ImageType, ValueType::Trigger],
            ValueType::NoiseWorleyDistanceFunction => {
                vec![ValueType::NoiseWorleyDistanceFunction, ValueType::Trigger]
            }
            ValueType::ColorSpace => vec![ValueType::ColorSpace, ValueType::Trigger],
            ValueType::BlendMode => vec![ValueType::BlendMode, ValueType::Trigger],
            ValueType::TextHAlign => vec![ValueType::TextHAlign, ValueType::Trigger],
            ValueType::TextVAlign => vec![ValueType::TextVAlign, ValueType::Trigger],
            ValueType::VideoContainer => vec![ValueType::VideoContainer, ValueType::Trigger],
            ValueType::VideoCodec => vec![ValueType::VideoCodec, ValueType::Trigger],
            ValueType::Video => vec![ValueType::Video, ValueType::Trigger],
        }
    }

    /// Return the list of types that can be converted **into** this type.
    ///
    /// This is the inverse of [`valid_conversions`](ValueType::valid_conversions):
    /// it finds all types whose valid_conversions list includes `self`.
    pub fn valid_conversions_from(&self) -> Vec<ValueType> {
        let mut types: Vec<ValueType> = Vec::new();

        for value_type in ValueType::types().iter() {
            //if value_type != self {
            if value_type.valid_conversions().contains(self) {
                types.push(value_type.clone());
            }
            //}
        }

        types
    }
}

/// Error returned when a value conversion fails (unsupported or runtime parse error).
#[derive(Debug)]
pub struct ConversionError {
    /// Human-readable description of what went wrong.
    pub message: String,
}

/// Pixel color format for image output encoding.
///
/// Maps to `image::ColorType` variants and controls the bit depth and channel
/// layout when saving images to disk.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColorFormat {
    Rgba32F,
    Rgb32F,
    Rgba16,
    Rgb16,
    GrayA16,
    Gray16,
    Rgba8,
    Rgb8,
    GrayA8,
    Gray8,
}

impl ColorFormat {
    /// Convert to the corresponding `image::ColorType` for encoding.
    pub fn format(&self) -> image::ColorType {
        match self {
            ColorFormat::Rgba32F => image::ColorType::Rgba32F,
            ColorFormat::Rgb32F => image::ColorType::Rgb32F,
            ColorFormat::Rgba16 => image::ColorType::Rgba16,
            ColorFormat::Rgb16 => image::ColorType::Rgb16,
            ColorFormat::GrayA16 => image::ColorType::La16,
            ColorFormat::Gray16 => image::ColorType::L16,
            ColorFormat::Rgba8 => image::ColorType::Rgba8,
            ColorFormat::Rgb8 => image::ColorType::Rgb8,
            ColorFormat::GrayA8 => image::ColorType::La8,
            ColorFormat::Gray8 => image::ColorType::L8,
        }
    }

    /// Return all available color format variants.
    pub fn types() -> [ColorFormat; 10] {
        let types: [ColorFormat; 10] = [
            ColorFormat::Rgba32F,
            ColorFormat::Rgb32F,
            ColorFormat::Rgba16,
            ColorFormat::Rgb16,
            ColorFormat::GrayA16,
            ColorFormat::Gray16,
            ColorFormat::Rgba8,
            ColorFormat::Rgb8,
            ColorFormat::GrayA8,
            ColorFormat::Gray8,
        ];

        types
    }

    /// Return a sensible default color format for the given image file format.
    ///
    /// Used to auto-correct the color format when the user picks an image format
    /// that doesn't support the currently selected color format.
    pub fn default_for_image_format(image_format: &image::ImageFormat) -> ColorFormat {
        match image_format {
            image::ImageFormat::OpenExr => ColorFormat::Rgba32F,
            image::ImageFormat::Farbfeld => ColorFormat::Rgba16,
            image::ImageFormat::Jpeg | image::ImageFormat::Bmp | image::ImageFormat::Pnm => ColorFormat::Rgb8,
            image::ImageFormat::Hdr => ColorFormat::Rgb8, // HDR is read-only but need a fallback
            _ => ColorFormat::Rgba8,
        }
    }

    /// Check whether this color format is compatible with the given image file format.
    ///
    /// Different image encoders support different bit depths and channel layouts.
    /// For example, JPEG only supports 8-bit without alpha, OpenEXR only supports
    /// 32-bit float, and Farbfeld only supports Rgba16.
    pub fn is_compatible_with_image_format(&self, image_format: &image::ImageFormat) -> bool {
        match image_format {
            // OpenEXR supports 32-bit float only
            image::ImageFormat::OpenExr => matches!(self, ColorFormat::Rgba32F | ColorFormat::Rgb32F),
            // Farbfeld supports Rgba16 only
            image::ImageFormat::Farbfeld => matches!(self, ColorFormat::Rgba16),
            // JPEG supports 8-bit without alpha only
            image::ImageFormat::Jpeg => matches!(self, ColorFormat::Rgb8 | ColorFormat::Gray8),
            // PNG and TIFF support 8-bit and 16-bit (no 32F)
            image::ImageFormat::Png | image::ImageFormat::Tiff => {
                !matches!(self, ColorFormat::Rgba32F | ColorFormat::Rgb32F)
            }
            // BMP and PNM support 8-bit, no alpha
            image::ImageFormat::Bmp | image::ImageFormat::Pnm => {
                matches!(self, ColorFormat::Rgb8 | ColorFormat::Gray8)
            }
            // GIF, WebP, TGA, ICO, QOI support 8-bit only
            image::ImageFormat::Gif
            | image::ImageFormat::WebP
            | image::ImageFormat::Tga
            | image::ImageFormat::Ico
            | image::ImageFormat::Qoi => {
                matches!(
                    self,
                    ColorFormat::Rgba8 | ColorFormat::Rgb8 | ColorFormat::GrayA8 | ColorFormat::Gray8
                )
            }
            // HDR is read-only
            image::ImageFormat::Hdr => false,
            // Unknown/other formats — allow and let the encoder decide
            _ => true,
        }
    }
}

/// Supported image file formats for reading and writing.
///
/// See <https://docs.rs/image/latest/image/codecs/index.html#supported-formats>
/// for the upstream codec support matrix.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ImageType {
    Png,
    Jpeg,
    Gif,
    WebP,
    Pnm,
    Tiff,
    Tga,
    //Dds,  // can't read or write
    Bmp,
    Ico,
    Hdr, // can't write
    OpenExr,
    Farbfeld,
    // Avif, Decoding requires the avif-native feature, uses the libdav1d C library.
    Qoi,
}

impl ImageType {
    /// Convert to the corresponding `image::ImageFormat` for encoding/decoding.
    pub fn format(&self) -> image::ImageFormat {
        match self {
            ImageType::Png => image::ImageFormat::Png,
            ImageType::Jpeg => image::ImageFormat::Jpeg,
            ImageType::Gif => image::ImageFormat::Gif,
            ImageType::WebP => image::ImageFormat::WebP,
            ImageType::Pnm => image::ImageFormat::Pnm,
            ImageType::Tiff => image::ImageFormat::Tiff,
            ImageType::Tga => image::ImageFormat::Tga,
            ImageType::Bmp => image::ImageFormat::Bmp,
            ImageType::Ico => image::ImageFormat::Ico,
            ImageType::Hdr => image::ImageFormat::Hdr,
            ImageType::OpenExr => image::ImageFormat::OpenExr,
            ImageType::Farbfeld => image::ImageFormat::Farbfeld,
            ImageType::Qoi => image::ImageFormat::Qoi,
        }
    }

    /// Return all available image type variants.
    pub fn types() -> [ImageType; 13] {
        let types: [ImageType; 13] = [
            ImageType::Png,
            ImageType::Jpeg,
            ImageType::Gif,
            ImageType::WebP,
            ImageType::Pnm,
            ImageType::Tiff,
            ImageType::Tga,
            ImageType::Bmp,
            ImageType::Ico,
            ImageType::Hdr,
            ImageType::OpenExr,
            ImageType::Farbfeld,
            ImageType::Qoi,
        ];

        types
    }
}

/// The wrapper file format of a video. Independent of the codec that
/// compresses the actual stream — a single container can carry many different
/// codecs (`Mkv + H264`, `Mkv + Vp9`, ...), subject to per-container support.
///
/// Used on both the video-input node (to report what the file is) and the
/// video-output node (to select the encoding target).
///
/// `#[repr(u8)]` is set so the discriminant can be used directly in
/// `Value::fingerprint` without a format!() hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum VideoContainer {
    Mp4,
    Mov,
    Mkv,
    WebM,
    Avi,
}

impl VideoContainer {
    /// Return the canonical file extension (without dot).
    pub fn extension(&self) -> &'static str {
        match self {
            VideoContainer::Mp4 => "mp4",
            VideoContainer::Mov => "mov",
            VideoContainer::Mkv => "mkv",
            VideoContainer::WebM => "webm",
            VideoContainer::Avi => "avi",
        }
    }

    /// Return every variant, for dropdown enumeration.
    pub fn types() -> [VideoContainer; 5] {
        [
            VideoContainer::Mp4,
            VideoContainer::Mov,
            VideoContainer::Mkv,
            VideoContainer::WebM,
            VideoContainer::Avi,
        ]
    }

    /// List of codecs this container legally carries. Single source of truth
    /// for the compatibility matrix — see `test_container_codec_matrix`.
    pub fn supported_codecs(&self) -> &'static [VideoCodec] {
        use VideoCodec::*;
        match self {
            VideoContainer::Mp4 => &[H264, H265, Av1, Mpeg4],
            VideoContainer::Mov => &[H264, H265, Mpeg4, ProRes],
            VideoContainer::Mkv => &[H264, H265, Vp8, Vp9, Av1, Mpeg4, ProRes],
            VideoContainer::WebM => &[Vp8, Vp9, Av1],
            VideoContainer::Avi => &[H264, Mpeg4],
        }
    }
}

/// The codec that compresses a video stream. Orthogonal to [`VideoContainer`];
/// not every codec fits every container — use
/// [`VideoCodec::is_supported_in`] or [`VideoContainer::supported_codecs`] to
/// validate a pair.
///
/// `#[repr(u8)]` is set so the discriminant can be used directly in
/// `Value::fingerprint` without a format!() hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum VideoCodec {
    H264,
    H265,
    Vp8,
    Vp9,
    Av1,
    Mpeg4,
    ProRes,
}

impl VideoCodec {
    /// Return every variant, for dropdown enumeration.
    pub fn types() -> [VideoCodec; 7] {
        [
            VideoCodec::H264,
            VideoCodec::H265,
            VideoCodec::Vp8,
            VideoCodec::Vp9,
            VideoCodec::Av1,
            VideoCodec::Mpeg4,
            VideoCodec::ProRes,
        ]
    }

    /// Whether this codec can legally be muxed into the given container.
    pub fn is_supported_in(&self, container: VideoContainer) -> bool {
        container.supported_codecs().contains(self)
    }
}

/// Immutable metadata extracted from a video file on open. Carried inside
/// [`VideoRef`] so downstream nodes can read width/height/fps/etc. without
/// re-querying the decoder cache.
///
/// Lives in `value.rs` (rather than the feature-gated `video` module) so
/// `Value::Video` is usable in builds without the `video` feature — saved
/// graphs containing video nodes still deserialize; they just can't run.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VideoMeta {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub duration_seconds: f64,
    pub total_frames: u32,
    pub container: VideoContainer,
    pub codec: VideoCodec,
}

impl Default for VideoMeta {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            fps: 0.0,
            duration_seconds: 0.0,
            total_frames: 0,
            container: VideoContainer::Mp4,
            codec: VideoCodec::H264,
        }
    }
}

/// A metadata-level transform applied to a [`VideoRef`].
///
/// Transforms compose left-to-right (in `VideoRef::transforms` order) to
/// build the effective timeline seen by downstream nodes. None of these
/// touch pixels — they only remap how an effective time/frame maps back
/// to a source frame index when an extract-frame op decodes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VideoTransformOp {
    /// Restrict playback to source seconds `[start, end]`.
    Trim { start_seconds: f64, end_seconds: f64 },
    /// Scale playback speed. `factor > 1.0` plays faster (shorter effective duration).
    Speed { factor: f32 },
    /// Play backwards.
    Reverse,
    /// Repeat the clip `count` times. `count = 1` is the identity.
    Loop { count: u32 },
}

impl VideoTransformOp {
    /// Apply this op in-place to a `VideoMeta`, updating `duration_seconds`
    /// and `total_frames`. All four current transforms are time-only, so
    /// width/height/fps/container/codec are preserved.
    pub fn apply_to_meta(&self, meta: &mut VideoMeta) {
        let new_dur = self.apply_to_duration(meta.duration_seconds);
        meta.duration_seconds = new_dur;
        meta.total_frames = if meta.fps > 0.0 {
            (new_dur * meta.fps as f64).round() as u32
        } else {
            0
        };
    }

    /// Effective duration produced by applying this op to a clip of
    /// `input_duration` seconds.
    pub fn apply_to_duration(&self, input_duration: f64) -> f64 {
        match *self {
            VideoTransformOp::Trim { start_seconds, end_seconds } => {
                let start = start_seconds.max(0.0).min(input_duration.max(0.0));
                let end = end_seconds.max(start).min(input_duration.max(0.0));
                end - start
            }
            VideoTransformOp::Speed { factor } => {
                if factor > 0.0 { input_duration / factor as f64 } else { input_duration }
            }
            VideoTransformOp::Reverse => input_duration,
            VideoTransformOp::Loop { count } => input_duration * count.max(1) as f64,
        }
    }

    /// Inverse time mapping. Given a time `t_out` in this op's output timeline
    /// (and the op's input-side duration), returns the corresponding time in
    /// its input timeline — i.e. peels one transform layer off.
    pub fn reverse_time(&self, t_out: f64, input_duration: f64) -> f64 {
        let input_duration = input_duration.max(0.0);
        match *self {
            VideoTransformOp::Trim { start_seconds, end_seconds } => {
                let start = start_seconds.max(0.0).min(input_duration);
                let end = end_seconds.max(start).min(input_duration);
                let clamped = t_out.max(0.0).min((end - start).max(0.0));
                (start + clamped).min(input_duration)
            }
            VideoTransformOp::Speed { factor } => {
                let k = if factor > 0.0 { factor as f64 } else { 1.0 };
                (t_out * k).max(0.0).min(input_duration)
            }
            VideoTransformOp::Reverse => {
                (input_duration - t_out).max(0.0).min(input_duration)
            }
            VideoTransformOp::Loop { count: _ } => {
                if input_duration > 0.0 {
                    t_out.rem_euclid(input_duration)
                } else {
                    0.0
                }
            }
        }
    }
}

/// Lightweight handle to a video file. Produced by the `video from file`
/// node and consumed by extract-frame ops; decoding state lives in the
/// global [`crate::video::VideoDecoderCache`] keyed by path.
///
/// Cloning is cheap — a `PathBuf`, two small `VideoMeta` copies, and a
/// small transform vector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoRef {
    /// Path to the underlying video file on disk.
    pub path: PathBuf,
    /// Effective metadata exposed to downstream nodes (fps, duration, total
    /// frames). Equals `source_meta` when `transforms` is empty; otherwise
    /// reflects the transform chain's composed effect.
    pub meta: VideoMeta,
    /// Original metadata read from the file on load. Needed to translate an
    /// effective-timeline query back to a source frame index for decoding.
    #[serde(default)]
    pub source_meta: VideoMeta,
    /// Metadata-level transform chain applied to the source. Empty = identity.
    #[serde(default)]
    pub transforms: Vec<VideoTransformOp>,
}

impl Default for VideoRef {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            meta: VideoMeta::default(),
            source_meta: VideoMeta::default(),
            transforms: Vec::new(),
        }
    }
}

impl VideoRef {
    /// Recompute `meta` from `source_meta` by walking `transforms` in order.
    /// Call after mutating the transform chain so downstream nodes see a
    /// consistent effective duration / total-frame count.
    pub fn recompute_effective_meta(&mut self) {
        let mut meta = self.source_meta;
        for op in &self.transforms {
            op.apply_to_meta(&mut meta);
        }
        self.meta = meta;
    }

    /// Append a transform and recompute `meta`. Convenience for the
    /// trim/speed/reverse/loop ops.
    pub fn with_transform(mut self, op: VideoTransformOp) -> Self {
        self.transforms.push(op);
        self.recompute_effective_meta();
        self
    }

    /// Map an effective-timeline second to a second in the source file's
    /// timeline by peeling the transform chain off in reverse application order.
    pub fn effective_to_source_seconds(&self, effective_seconds: f64) -> f64 {
        // Walk forward once to record the input-side duration for each op —
        // Reverse and Loop need it to invert their mapping.
        let mut durations_before = Vec::with_capacity(self.transforms.len());
        let mut cur = self.source_meta.duration_seconds;
        for op in &self.transforms {
            durations_before.push(cur);
            cur = op.apply_to_duration(cur);
        }

        let mut t = effective_seconds;
        for (op, d_in) in self.transforms.iter().zip(durations_before.iter()).rev() {
            t = op.reverse_time(t, *d_in);
        }
        t
    }

    /// Convert an effective-timeline second into a source frame index suitable
    /// for `VideoDecoderCache::frame`. Clamps to `[0, source_total_frames - 1]`.
    /// Returns `0` if the source has no fps (unknown / zero-fps clips).
    pub fn source_frame_for_effective_time(&self, effective_seconds: f64) -> u32 {
        let source_seconds = self.effective_to_source_seconds(effective_seconds);
        let src_fps = self.source_meta.fps as f64;
        if src_fps > 0.0 {
            let max_idx = self.source_meta.total_frames.saturating_sub(1) as i64;
            let idx = (source_seconds.max(0.0) * src_fps).round() as i64;
            idx.clamp(0, max_idx.max(0)) as u32
        } else {
            0
        }
    }

    /// Convert an effective-timeline frame index into a source frame index.
    /// Goes via effective-time using `meta.fps` so that frame→source remains
    /// consistent with time→source even when transforms change fps or duration.
    pub fn source_frame_for_effective_frame(&self, effective_frame: u32) -> u32 {
        let eff_fps = self.meta.fps as f64;
        if eff_fps > 0.0 {
            let t = effective_frame as f64 / eff_fps;
            self.source_frame_for_effective_time(t)
        } else {
            // Degenerate case: no effective fps. Treat the effective frame as
            // already being a source frame and clamp.
            let max_idx = self.source_meta.total_frames.saturating_sub(1);
            effective_frame.min(max_idx)
        }
    }
}

/// Horizontal alignment for multi-line text rendering.
///
/// Controls how each line is positioned relative to the anchor's x coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TextHAlign {
    /// Lines start at the anchor x (text extends rightward).
    Left,
    /// Lines are centred on the anchor x.
    Center,
    /// Lines end at the anchor x (text extends leftward).
    Right,
}

impl TextHAlign {
    /// Returns all horizontal alignment variants in display order.
    pub fn types() -> [TextHAlign; 3] {
        [TextHAlign::Left, TextHAlign::Center, TextHAlign::Right]
    }
}

/// Vertical alignment for multi-line text rendering.
///
/// Controls how the text block is positioned relative to the anchor's y coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TextVAlign {
    /// Block top aligns with the anchor y (text extends downward).
    Top,
    /// Block is centred on the anchor y.
    Middle,
    /// Block bottom aligns with the anchor y (text extends upward).
    Bottom,
}

impl TextVAlign {
    /// Returns all vertical alignment variants in display order.
    pub fn types() -> [TextVAlign; 3] {
        [TextVAlign::Top, TextVAlign::Middle, TextVAlign::Bottom]
    }
}

/// A UI button state wrapper (pressed or not).
#[derive(Debug, Clone)]
pub struct UiButton(pub bool);

/// Custom serializer for `FilterType` since it doesn't implement `Serialize`.
fn serialize_filter_type<S>(value: &FilterType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let serialized_value = match value {
        FilterType::CatmullRom => "catmullrom",
        FilterType::Gaussian => "gaussian",
        FilterType::Lanczos3 => "lanczos3",
        FilterType::Nearest => "nearest",
        FilterType::Triangle => "triangle",
    };
    serializer.serialize_str(serialized_value)
}

/// Custom deserializer for `FilterType`. Accepts legacy "guassian" typo for backwards compatibility.
fn deserialize_filter_type<'de, D>(deserializer: D) -> Result<FilterType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let deserialized_value = String::deserialize(deserializer)?;
    match deserialized_value.as_str() {
        "catmullrom" => Ok(FilterType::CatmullRom),
        "gaussian" | "guassian" => Ok(FilterType::Gaussian),
        "lanczos3" => Ok(FilterType::Lanczos3),
        "nearest" => Ok(FilterType::Nearest),
        "triangle" => Ok(FilterType::Triangle),
        _ => Err(serde::de::Error::custom("Unknown enum value")),
    }
}

/// Custom serializer for `image::ImageFormat`, encoding as the first file extension string.
fn serialize_image_format<S>(value: &image::ImageFormat, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let serialized_value = value.extensions_str()[0];
    serializer.serialize_str(serialized_value)
}

/// Custom deserializer for `image::ImageFormat`, looking up format by file extension string.
fn deserialize_image_format<'de, D>(deserializer: D) -> Result<image::ImageFormat, D::Error>
where
    D: serde::Deserializer<'de>,
{
    if let Ok(s) = String::deserialize(deserializer) {
        if let Some(format) = image::ImageFormat::from_extension(s) {
            Ok(format)
        } else {
            Err(serde::de::Error::custom("Unknown enum value"))
        }
    } else {
        Err(serde::de::Error::custom("Unknown enum value"))
    }
}

/// Custom serde module that replaces image values with a tiny 1x1
/// placeholder during serialization to avoid storing full image pixel data in
/// save files. Non-image values pass through unchanged. On deserialization,
/// values are read normally (the 1x1 placeholder is a valid image).
pub mod value_skip_images {
    use super::Value;
    use crate::float_image::FloatImage;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::sync::Arc;
    use crate::get_id;

    /// Serializes a `Value`, replacing any Image with a 1x1 placeholder.
    pub fn serialize<S>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Value::Image { .. } => {
                // Replace with a tiny 1x1 placeholder to preserve the variant tag
                // without storing megabytes of pixel data.
                let placeholder = Value::Image {
                    data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[1.0, 1.0, 1.0, 1.0])),
                    change_id: get_id(),
                };
                placeholder.serialize(serializer)
            }
            other => other.serialize(serializer),
        }
    }

    /// Deserializes a `Value` normally (the placeholder is a valid image).
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Value::deserialize(deserializer)
    }
}

#[cfg(test)]
#[path = "value_tests.rs"]
mod tests;
