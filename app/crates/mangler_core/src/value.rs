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
    operations::images::noise::cellular::worley_distance::NoiseWorleyDistanceFunction,
    thumbnail::Thumbnail,
};

/// Dimensions (width, height) used when generating thumbnail previews.
pub const THUMBNAIL_SIZE: [u32; 2] = [150, 150];

/// Maximum characters kept in a text thumbnail. The UI wraps and elides
/// further; this just keeps arbitrarily long strings out of the messages.
pub const THUMBNAIL_TEXT_MAX_CHARS: usize = 300;

/// Truncate `text` to [`THUMBNAIL_TEXT_MAX_CHARS`], appending `…` if cut.
fn truncate_thumbnail_text(text: &str) -> String {
    match text.char_indices().nth(THUMBNAIL_TEXT_MAX_CHARS) {
        Some((byte_index, _)) => format!("{}…", &text[..byte_index]),
        None => text.to_string(),
    }
}

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
    /// Edge-fill mode for image transforms (fill / wrap / extend / mirror).
    EdgeMode(EdgeMode),
    /// Horizontal alignment for text rendering.
    TextHAlign(TextHAlign),
    /// Vertical alignment for text rendering.
    TextVAlign(TextVAlign),
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
                let img = RgbaImage::from_pixel(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], color);

                Some(Thumbnail::Image(img))
            }
            Value::Image { data, change_id: _ } => Some(Thumbnail::Image(
                data.resize_fit(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]).to_rgba8(),
            )),
            Value::Bool(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Integer(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Decimal(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::Text(value) => Some(Thumbnail::Text(truncate_thumbnail_text(value))),
            Value::Path(path) => Some(Thumbnail::Text(truncate_thumbnail_text(
                path.to_str().unwrap_or("none"),
            ))),
            Value::FilterType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::ColorFormat(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::Trigger => Some(Thumbnail::Text("trigger".to_string())),
            Value::ImageType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::NoiseWorleyDistanceFunction(value) => {
                Some(Thumbnail::Text(format!("{:?}", value)))
            }
            Value::ColorSpace(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::BlendMode(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::EdgeMode(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::TextHAlign(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::TextVAlign(value) => Some(Thumbnail::Text(format!("{:?}", value))),
        }
    }

    /// Allocation-free fingerprint for cache comparison.
    /// Returns a u64 hash that changes when the value changes. Hashes are
    /// runtime-only cache keys (never persisted), so the exact values may
    /// change between builds. Enum-valued variants hash the inner enum's
    /// `mem::discriminant` — all of those enums are fieldless, so the
    /// discriminant fully identifies the value without allocating.
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
            Value::ImageType(it) => std::mem::discriminant(it).hash(&mut h),
            // Always the same: a Trigger carries no data, so its hash cannot
            // encode freshness. Graph::run compensates with its forced_nodes
            // set — trigger firings bypass the input-hash skip there.
            Value::Trigger => 0u8.hash(&mut h),
            Value::NoiseWorleyDistanceFunction(w) => std::mem::discriminant(w).hash(&mut h),
            Value::ColorSpace(cs) => std::mem::discriminant(cs).hash(&mut h),
            Value::BlendMode(bm) => std::mem::discriminant(bm).hash(&mut h),
            Value::EdgeMode(v) => std::mem::discriminant(v).hash(&mut h),
            Value::TextHAlign(v) => std::mem::discriminant(v).hash(&mut h),
            Value::TextVAlign(v) => std::mem::discriminant(v).hash(&mut h),
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
            Value::EdgeMode(_) => ValueType::EdgeMode,
            Value::TextHAlign(_) => ValueType::TextHAlign,
            Value::TextVAlign(_) => ValueType::TextVAlign,
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
            Value::Trigger => match other {
                ValueType::Trigger => Ok(Value::Trigger),
                _ => Err(ConversionError {
                    message: "Unable to convert trigger to this type.".to_string(),
                }),
            },
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
                    Ok(Value::NoiseWorleyDistanceFunction(*a))
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
            Value::EdgeMode(a) => match other {
                ValueType::EdgeMode => Ok(Value::EdgeMode(*a)),
                _ => Err(ConversionError { message: "Unable to convert.".to_string() }),
            },
            Value::TextHAlign(a) => match other {
                ValueType::TextHAlign => Ok(Value::TextHAlign(*a)),
                _ => Err(ConversionError { message: "Unable to convert.".to_string() }),
            },
            Value::TextVAlign(a) => match other {
                ValueType::TextVAlign => Ok(Value::TextVAlign(*a)),
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
                        "chebyshev" => Ok(Value::NoiseWorleyDistanceFunction(crate::operations::images::noise::cellular::worley_distance::NoiseWorleyDistanceFunction::Chebyshev)),
                        "euclidean" => Ok(Value::NoiseWorleyDistanceFunction(crate::operations::images::noise::cellular::worley_distance::NoiseWorleyDistanceFunction::Euclidean)),
                        "euclideansquared" | "euclidean_squared" | "euclidean squared" => Ok(Value::NoiseWorleyDistanceFunction(crate::operations::images::noise::cellular::worley_distance::NoiseWorleyDistanceFunction::EuclideanSquared)),
                        "manhattan" => Ok(Value::NoiseWorleyDistanceFunction(crate::operations::images::noise::cellular::worley_distance::NoiseWorleyDistanceFunction::Manhattan)),
                        "quadratic" => Ok(Value::NoiseWorleyDistanceFunction(crate::operations::images::noise::cellular::worley_distance::NoiseWorleyDistanceFunction::Quadratic)),
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
    /// Edge-fill mode type for image transforms.
    EdgeMode,
    /// Horizontal text alignment type.
    TextHAlign,
    /// Vertical text alignment type.
    TextVAlign,
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
            ValueType::EdgeMode => Value::EdgeMode(EdgeMode::Fill),
            ValueType::TextHAlign => Value::TextHAlign(TextHAlign::Center),
            ValueType::TextVAlign => Value::TextVAlign(TextVAlign::Middle),
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
            ValueType::EdgeMode => "edge mode".to_string(),
            ValueType::TextHAlign => "text h-align".to_string(),
            ValueType::TextVAlign => "text v-align".to_string(),
        }
    }

    /// Return the file extensions that can be opened for each value type.
    /// Currently only `DynamicImage` has associated file extensions.
    pub fn file_extensions(value_type: &ValueType) -> Vec<String> {
        match value_type {
            ValueType::Image => {
                let mut list = vec![];

                for image_format in ImageType::types().iter() {
                    // AVIF is write-only: decoding needs the avif-native C library.
                    if matches!(image_format, ImageType::Avif) {
                        continue;
                    }
                    let ext = image_format.format().extensions_str()[0];
                    list.push(ext.to_string());
                }

                // Formats decoded outside the image crate (see the image
                // "from file" operation): JPEG XL via jxl-oxide, PSD via psd.
                list.push("jxl".to_string());
                list.push("psd".to_string());

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
            ValueType::EdgeMode => vec![ValueType::EdgeMode, ValueType::Trigger],
            ValueType::TextHAlign => vec![ValueType::TextHAlign, ValueType::Trigger],
            ValueType::TextVAlign => vec![ValueType::TextVAlign, ValueType::Trigger],
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
            image::ImageFormat::Hdr => ColorFormat::Rgb32F,
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
            // GIF, WebP, TGA, ICO, QOI, AVIF support 8-bit only (the AVIF
            // encoder accepts 16-bit input but always encodes 8-bit)
            image::ImageFormat::Gif
            | image::ImageFormat::WebP
            | image::ImageFormat::Tga
            | image::ImageFormat::Ico
            | image::ImageFormat::Qoi
            | image::ImageFormat::Avif => {
                matches!(
                    self,
                    ColorFormat::Rgba8 | ColorFormat::Rgb8 | ColorFormat::GrayA8 | ColorFormat::Gray8
                )
            }
            // Radiance HDR encodes RGBE, written from 32-bit float RGB only
            image::ImageFormat::Hdr => matches!(self, ColorFormat::Rgb32F),
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
    Hdr, // write requires Rgb32F
    OpenExr,
    Farbfeld,
    Avif, // write-only: decoding needs the avif-native feature (libdav1d C library)
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
            ImageType::Avif => image::ImageFormat::Avif,
            ImageType::Qoi => image::ImageFormat::Qoi,
        }
    }

    /// Return all available image type variants. Every listed type can be
    /// written; all except AVIF can also be read.
    pub fn types() -> [ImageType; 14] {
        let types: [ImageType; 14] = [
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
            ImageType::Avif,
            ImageType::Qoi,
        ];

        types
    }
}

/// How an image transform fills the space it exposes (translate/rotate/scale).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EdgeMode {
    /// Exposed space is filled with a solid colour (transparent by default).
    Fill,
    /// The image tiles: content off one side reappears on the other.
    Wrap,
    /// Border pixels are stretched out to the edge.
    Extend,
    /// The image is reflected back across each edge.
    Mirror,
}

impl EdgeMode {
    /// Returns all edge modes in display order (matches dropdown ordering).
    pub fn types() -> [EdgeMode; 4] {
        [EdgeMode::Fill, EdgeMode::Wrap, EdgeMode::Extend, EdgeMode::Mirror]
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
