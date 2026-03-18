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

use image::{imageops::FilterType, DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::{
    color::Color, get_id, operations::images::noise::worley_distance::NoiseWorleyDistanceFunction,
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
    String(String),
    /// An sRGBA color with floating-point channels.
    Color(Color),
    /// A full raster image wrapped in an `Arc` for cheap cloning.
    DynamicImage {
        /// The image data, reference-counted for efficient sharing.
        #[serde(with = "crate::dynamic_image_serde")]
        data: Arc<DynamicImage>,
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
            //Value::DynamicImage { data, change_id:_ } => Some(Thumbnail::Image(data.thumbnail(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]).into_rgba8())),
            Value::DynamicImage { data, change_id: _ } => Some(Thumbnail::Image(
                data.thumbnail(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1])
                    .to_rgba8(),
            )),
            Value::Bool(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Integer(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Decimal(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::String(value) => Some(Thumbnail::Text(value.clone())),
            Value::Path(path) => Some(Thumbnail::Text(format!(
                "{}",
                path.to_str().unwrap_or("none").to_string()
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
            Value::String(v) => v.hash(&mut h),
            Value::Color(c) => {
                c.r.to_bits().hash(&mut h);
                c.g.to_bits().hash(&mut h);
                c.b.to_bits().hash(&mut h);
                c.a.to_bits().hash(&mut h);
            }
            Value::DynamicImage { data: _, change_id } => change_id.hash(&mut h),
            Value::Path(p) => p.hash(&mut h),
            Value::FilterType(f) => (*f as u8).hash(&mut h),
            Value::ColorFormat(cf) => (*cf as u8).hash(&mut h),
            Value::ImageType(it) => format!("{:?}", it).hash(&mut h),
            Value::Trigger => 0u8.hash(&mut h), // always same — triggers re-run via is_dirty
            Value::NoiseWorleyDistanceFunction(w) => format!("{:?}", w).hash(&mut h),
            Value::ColorSpace(cs) => format!("{:?}", cs).hash(&mut h),
            Value::BlendMode(bm) => format!("{:?}", bm).hash(&mut h),
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
            Value::String(_) => ValueType::String,
            Value::Color(_) => ValueType::Color,
            Value::ColorFormat(_) => ValueType::ColorFormat,
            Value::Trigger => ValueType::Trigger,
            Value::FilterType(_) => ValueType::FilterType,
            Value::Path(_) => ValueType::Path,
            Value::DynamicImage {
                data: _,
                change_id: _,
            } => ValueType::DynamicImage,
            Value::ImageType(_) => ValueType::ImageType,
            Value::NoiseWorleyDistanceFunction(_) => ValueType::NoiseWorleyDistanceFunction,
            Value::ColorSpace(_) => ValueType::ColorSpace,
            Value::BlendMode(_) => ValueType::BlendMode,
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
                ValueType::String => Ok(Value::String(a.to_string())),
                ValueType::Color => {
                    if *a {
                        Ok(Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)))
                    } else {
                        Ok(Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)))
                    }
                }
                ValueType::DynamicImage => {
                    let mut imgbuf = image::RgbaImage::new(1, 1);
                    let color_value: u8 = if *a { 255 } else { 0 };

                    for (_x, _y, pixel) in imgbuf.enumerate_pixels_mut() {
                        *pixel = image::Rgba([color_value, color_value, color_value, color_value]);
                    }

                    Ok(Value::DynamicImage {
                        data: Arc::new(DynamicImage::ImageRgba8(imgbuf)),
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
                ValueType::String => Ok(Value::String(a.to_string())),
                ValueType::Color => {
                    let v = (*a).clamp(0, 255) as f32 / 255.0;
                    Ok(Value::Color(Color::from_srgb_float(v, v, v, 1.0)))
                }
                ValueType::DynamicImage => {
                    let v = (*a).clamp(0, 255) as u8;
                    let imgbuf = image::RgbaImage::from_pixel(1, 1, image::Rgba([v, v, v, 255]));
                    Ok(Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() })
                }
                _ => Err(ConversionError {
                    message: "Unable to convert integer to this type.".to_string(),
                }),
            },
            Value::Decimal(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a != 0.0)),
                ValueType::Integer => Ok(Value::Integer(*a as i32)),
                ValueType::Decimal => Ok(Value::Decimal(*a)),
                ValueType::String => Ok(Value::String(a.to_string())),
                ValueType::Color => {
                    let v = a.clamp(0.0, 1.0);
                    Ok(Value::Color(Color::from_srgb_float(v, v, v, 1.0)))
                }
                ValueType::DynamicImage => {
                    let v = (a.clamp(0.0, 1.0) * 255.0) as u8;
                    let imgbuf = image::RgbaImage::from_pixel(1, 1, image::Rgba([v, v, v, 255]));
                    Ok(Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() })
                }
                _ => Err(ConversionError {
                    message: "Unable to convert decimal to this type.".to_string(),
                }),
            },
            Value::String(a) => match other {
                ValueType::Bool => {
                    let result: Result<bool, _> = a.parse();
                    match result {
                        Ok(r) => Ok(Value::Bool(r)),
                        Err(_) => Err(ConversionError {
                            message: "Error converting string to bool.".to_string(),
                        }),
                    }
                }
                ValueType::Integer => {
                    let result: Result<i32, _> = a.parse();
                    match result {
                        Ok(r) => Ok(Value::Integer(r)),
                        Err(_) => Err(ConversionError {
                            message: "Error converting string to integer.".to_string(),
                        }),
                    }
                }
                ValueType::Decimal => {
                    let result: Result<f32, _> = a.parse();
                    match result {
                        Ok(r) => Ok(Value::Decimal(r)),
                        Err(_) => Err(ConversionError {
                            message: "Error converting string to decimal.".to_string(),
                        }),
                    }
                }
                ValueType::String => Ok(Value::String(a.clone())),
                ValueType::Path => Ok(Value::Path(PathBuf::from(a))),
                _ => Err(ConversionError {
                    message: "Unable to convert string to this type.".to_string(),
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
                ValueType::String => Ok(Value::String(format!("rgba({}, {}, {}, {})", a.r, a.g, a.b, a.a))),
                ValueType::Color => Ok(Value::Color(*a)),
                ValueType::DynamicImage => {
                    let rgba = a.to_srgb_u8();
                    let imgbuf = image::RgbaImage::from_pixel(1, 1, image::Rgba([rgba.0, rgba.1, rgba.2, rgba.3]));
                    Ok(Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() })
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
            Value::DynamicImage { data, change_id } => match other {
                ValueType::DynamicImage => Ok(Value::DynamicImage {
                    data: data.clone(),
                    change_id: change_id.clone(),
                }),
                _ => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            },
            Value::Path(path) => match other {
                ValueType::String => {
                    if let Ok(path_string) = path.clone().into_os_string().into_string() {
                        Ok(Value::String(path_string))
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
                ValueType::ImageType => Ok(Value::ImageType(image_format.clone())),
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
                ValueType::ColorSpace => Ok(Value::ColorSpace(a.clone())),
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
    /// UTF-8 string type.
    String,
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
    /// Raster image type.
    DynamicImage,
    /// Filesystem path type.
    Path,
    /// Worley noise distance function type.
    NoiseWorleyDistanceFunction,
    /// Color space identifier type.
    ColorSpace,
    /// Blend mode type.
    BlendMode,
}

impl ValueType {
    /// Return the standard set of value types available for general use.
    pub fn types() -> [ValueType; 10] {
        let types: [ValueType; 10] = [
            ValueType::Bool,
            ValueType::Integer,
            ValueType::Decimal,
            ValueType::String,
            ValueType::Color,
            ValueType::FilterType,
            ValueType::ColorFormat,
            ValueType::Trigger,
            ValueType::DynamicImage,
            ValueType::Path,
        ];

        types
    }

    /// Return a human-readable name for this type, used in the UI.
    pub fn value_name(&self) -> String {
        match self {
            ValueType::Bool => "bool".to_string(),
            ValueType::Integer => "integer".to_string(),
            ValueType::Decimal => "decimal".to_string(),
            ValueType::String => "string".to_string(),
            ValueType::Color => "color".to_string(),
            ValueType::FilterType => "filter type".to_string(),
            ValueType::ColorFormat => "color format".to_string(),
            ValueType::Trigger => "trigger".to_string(),
            ValueType::DynamicImage => "image".to_string(),
            ValueType::Path => "path".to_string(),
            ValueType::ImageType => "image format".to_string(),
            ValueType::NoiseWorleyDistanceFunction => "worley noise distance function".to_string(),
            ValueType::ColorSpace => "color space".to_string(),
            ValueType::BlendMode => "blend mode".to_string(),
        }
    }

    /// Return the file extensions that can be opened for each value type.
    /// Currently only `DynamicImage` has associated file extensions.
    pub fn file_extensions(value_type: &ValueType) -> Vec<String> {
        match value_type {
            ValueType::DynamicImage => {
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
                ValueType::String,
                ValueType::Trigger,
            ],
            ValueType::Integer => vec![
                ValueType::Bool,
                ValueType::Integer,
                ValueType::Decimal,
                ValueType::String,
                ValueType::Color,
                ValueType::DynamicImage,
                ValueType::Trigger,
            ],
            ValueType::Decimal => vec![
                ValueType::Bool,
                ValueType::Integer,
                ValueType::Decimal,
                ValueType::String,
                ValueType::Color,
                ValueType::DynamicImage,
                ValueType::Trigger,
            ],
            ValueType::String => vec![ValueType::String, ValueType::Path, ValueType::Trigger],
            ValueType::Color => vec![
                ValueType::Bool,
                ValueType::Integer,
                ValueType::Decimal,
                ValueType::String,
                ValueType::Color,
                ValueType::DynamicImage,
                ValueType::Trigger,
            ],
            ValueType::DynamicImage => vec![ValueType::DynamicImage, ValueType::Trigger],
            ValueType::Path => vec![ValueType::String, ValueType::Path, ValueType::Trigger],
            ValueType::FilterType => {
                vec![ValueType::FilterType, ValueType::String, ValueType::Trigger]
            }
            ValueType::ColorFormat => vec![
                ValueType::ColorFormat,
                ValueType::String,
                ValueType::Trigger,
            ],
            ValueType::Trigger => vec![ValueType::Trigger],
            ValueType::ImageType => vec![ValueType::ImageType, ValueType::Trigger],
            ValueType::NoiseWorleyDistanceFunction => {
                vec![ValueType::NoiseWorleyDistanceFunction, ValueType::Trigger]
            }
            ValueType::ColorSpace => vec![ValueType::ColorSpace, ValueType::Trigger],
            ValueType::BlendMode => vec![ValueType::BlendMode, ValueType::Trigger],
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
            if value_type.valid_conversions().contains(&self) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Helper to match Value variants since Value doesn't impl PartialEq
    macro_rules! assert_value {
        ($val:expr, Bool($expected:expr)) => {
            match &$val {
                Value::Bool(v) => assert_eq!(*v, $expected),
                other => panic!("Expected Bool({}), got {:?}", $expected, other),
            }
        };
        ($val:expr, Integer($expected:expr)) => {
            match &$val {
                Value::Integer(v) => assert_eq!(*v, $expected),
                other => panic!("Expected Integer({}), got {:?}", $expected, other),
            }
        };
        ($val:expr, Decimal($expected:expr)) => {
            match &$val {
                Value::Decimal(v) => assert!(
                    (*v - $expected).abs() < 1e-6,
                    "Expected Decimal({}), got Decimal({})",
                    $expected,
                    v
                ),
                other => panic!("Expected Decimal({}), got {:?}", $expected, other),
            }
        };
        ($val:expr, String($expected:expr)) => {
            match &$val {
                Value::String(v) => assert_eq!(v, $expected),
                other => panic!("Expected String({}), got {:?}", $expected, other),
            }
        };
    }

    // value_type tests
    #[test]
    fn test_value_type_bool() {
        assert_eq!(Value::Bool(true).value_type(), ValueType::Bool);
    }

    #[test]
    fn test_value_type_integer() {
        assert_eq!(Value::Integer(42).value_type(), ValueType::Integer);
    }

    #[test]
    fn test_value_type_decimal() {
        assert_eq!(Value::Decimal(3.14).value_type(), ValueType::Decimal);
    }

    #[test]
    fn test_value_type_string() {
        assert_eq!(
            Value::String("hi".to_string()).value_type(),
            ValueType::String
        );
    }

    #[test]
    fn test_value_type_color() {
        assert_eq!(
            Value::Color(Color::default()).value_type(),
            ValueType::Color
        );
    }

    #[test]
    fn test_value_type_path() {
        assert_eq!(Value::Path(PathBuf::new()).value_type(), ValueType::Path);
    }

    #[test]
    fn test_value_type_trigger() {
        assert_eq!(Value::Trigger.value_type(), ValueType::Trigger);
    }

    // try_convert_to: Bool conversions
    #[test]
    fn test_bool_true_to_integer() {
        let result = Value::Bool(true)
            .try_convert_to(ValueType::Integer)
            .unwrap();
        assert_value!(result, Integer(1));
    }

    #[test]
    fn test_bool_false_to_integer() {
        let result = Value::Bool(false)
            .try_convert_to(ValueType::Integer)
            .unwrap();
        assert_value!(result, Integer(0));
    }

    #[test]
    fn test_bool_true_to_decimal() {
        let result = Value::Bool(true)
            .try_convert_to(ValueType::Decimal)
            .unwrap();
        assert_value!(result, Decimal(1.0));
    }

    #[test]
    fn test_bool_false_to_decimal() {
        let result = Value::Bool(false)
            .try_convert_to(ValueType::Decimal)
            .unwrap();
        assert_value!(result, Decimal(0.0));
    }

    #[test]
    fn test_bool_to_string() {
        let result = Value::Bool(true).try_convert_to(ValueType::String).unwrap();
        assert_value!(result, String("true"));
    }

    #[test]
    fn test_bool_to_bool_identity() {
        let result = Value::Bool(true).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_bool_to_color_true() {
        let result = Value::Bool(true).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => {
                assert_eq!(c.r, 1.0);
                assert_eq!(c.g, 1.0);
                assert_eq!(c.b, 1.0);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_bool_to_color_false() {
        let result = Value::Bool(false).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => {
                assert_eq!(c.r, 0.0);
                assert_eq!(c.g, 0.0);
                assert_eq!(c.b, 0.0);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_bool_to_dynamic_image() {
        let result = Value::Bool(true).try_convert_to(ValueType::DynamicImage);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::DynamicImage {
                data: _,
                change_id: _,
            } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[test]
    fn test_bool_to_filter_type_fails() {
        let result = Value::Bool(true).try_convert_to(ValueType::FilterType);
        assert!(result.is_err());
    }

    // try_convert_to: Integer conversions
    #[test]
    fn test_integer_to_bool_nonzero() {
        let result = Value::Integer(42).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_integer_to_bool_zero() {
        let result = Value::Integer(0).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(false));
    }

    #[test]
    fn test_integer_to_decimal() {
        let result = Value::Integer(42)
            .try_convert_to(ValueType::Decimal)
            .unwrap();
        assert_value!(result, Decimal(42.0));
    }

    #[test]
    fn test_integer_to_string() {
        let result = Value::Integer(42)
            .try_convert_to(ValueType::String)
            .unwrap();
        assert_value!(result, String("42"));
    }

    #[test]
    fn test_integer_to_integer_identity() {
        let result = Value::Integer(42)
            .try_convert_to(ValueType::Integer)
            .unwrap();
        assert_value!(result, Integer(42));
    }

    #[test]
    fn test_integer_to_color_succeeds() {
        let result = Value::Integer(42).try_convert_to(ValueType::Color);
        assert!(result.is_ok());
    }

    // try_convert_to: Decimal conversions
    #[test]
    fn test_decimal_to_bool_nonzero() {
        let result = Value::Decimal(3.14)
            .try_convert_to(ValueType::Bool)
            .unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_decimal_to_bool_zero() {
        let result = Value::Decimal(0.0).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(false));
    }

    #[test]
    fn test_decimal_to_integer() {
        let result = Value::Decimal(3.14)
            .try_convert_to(ValueType::Integer)
            .unwrap();
        assert_value!(result, Integer(3));
    }

    #[test]
    fn test_decimal_to_string() {
        let result = Value::Decimal(3.14)
            .try_convert_to(ValueType::String)
            .unwrap();
        match result {
            Value::String(_) => {}
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_decimal_to_decimal_identity() {
        let result = Value::Decimal(3.14)
            .try_convert_to(ValueType::Decimal)
            .unwrap();
        assert_value!(result, Decimal(3.14));
    }

    // try_convert_to: String conversions
    #[test]
    fn test_string_to_bool_true() {
        let result = Value::String("true".to_string())
            .try_convert_to(ValueType::Bool)
            .unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_string_to_bool_false() {
        let result = Value::String("false".to_string())
            .try_convert_to(ValueType::Bool)
            .unwrap();
        assert_value!(result, Bool(false));
    }

    #[test]
    fn test_string_to_bool_invalid() {
        let result = Value::String("not a bool".to_string()).try_convert_to(ValueType::Bool);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_to_integer() {
        let result = Value::String("42".to_string())
            .try_convert_to(ValueType::Integer)
            .unwrap();
        assert_value!(result, Integer(42));
    }

    #[test]
    fn test_string_to_integer_invalid() {
        let result = Value::String("abc".to_string()).try_convert_to(ValueType::Integer);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_to_decimal() {
        let result = Value::String("3.14".to_string())
            .try_convert_to(ValueType::Decimal)
            .unwrap();
        assert_value!(result, Decimal(3.14));
    }

    #[test]
    fn test_string_to_decimal_invalid() {
        let result = Value::String("abc".to_string()).try_convert_to(ValueType::Decimal);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_to_string_identity() {
        let result = Value::String("hello".to_string())
            .try_convert_to(ValueType::String)
            .unwrap();
        assert_value!(result, String("hello"));
    }

    // try_convert_to: Other types
    #[test]
    fn test_color_to_color_identity() {
        let color = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
        let result = Value::Color(color)
            .try_convert_to(ValueType::Color)
            .unwrap();
        match result {
            Value::Color(c) => assert_eq!(c, color),
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_color_to_integer_succeeds() {
        let result = Value::Color(Color::default()).try_convert_to(ValueType::Integer);
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_to_string() {
        let result = Value::Path(PathBuf::from("/test/path"))
            .try_convert_to(ValueType::String)
            .unwrap();
        match result {
            Value::String(s) => assert!(s.contains("test")),
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_path_to_path_identity() {
        let result = Value::Path(PathBuf::from("/test"))
            .try_convert_to(ValueType::Path)
            .unwrap();
        match result {
            Value::Path(p) => assert_eq!(p, PathBuf::from("/test")),
            other => panic!("Expected Path, got {:?}", other),
        }
    }

    // === Edge cases: Decimal → Bool (truthiness) ===
    #[test]
    fn test_decimal_to_bool_small_positive() {
        // 0.1 is truthy (non-zero)
        let result = Value::Decimal(0.1).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_decimal_to_bool_small_negative() {
        // -0.1 is truthy (non-zero)
        let result = Value::Decimal(-0.1).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_decimal_to_bool_negative() {
        // -3.14 is truthy
        let result = Value::Decimal(-3.14).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_decimal_to_bool_one() {
        let result = Value::Decimal(1.0).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_decimal_to_bool_negative_zero() {
        // -0.0 == 0.0 in IEEE 754, so should be falsy
        let result = Value::Decimal(-0.0).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(false));
    }

    #[test]
    fn test_decimal_to_bool_very_small() {
        // f32::MIN_POSITIVE is truthy
        let result = Value::Decimal(f32::MIN_POSITIVE).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_decimal_to_bool_infinity() {
        let result = Value::Decimal(f32::INFINITY).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_decimal_to_bool_neg_infinity() {
        let result = Value::Decimal(f32::NEG_INFINITY).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_decimal_to_bool_nan() {
        // NaN != 0.0 is true, so NaN is truthy (matches JS: Boolean(NaN) === false... wait)
        // Actually in JS: Boolean(NaN) === false. But our code uses != 0.0, and NaN != 0.0 is true.
        // This documents the current behavior.
        let result = Value::Decimal(f32::NAN).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    // === Edge cases: Integer → Bool ===
    #[test]
    fn test_integer_to_bool_one() {
        let result = Value::Integer(1).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_integer_to_bool_negative() {
        // -1 is truthy (non-zero)
        let result = Value::Integer(-1).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_integer_to_bool_large_negative() {
        let result = Value::Integer(-999).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_integer_to_bool_max() {
        let result = Value::Integer(i32::MAX).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_integer_to_bool_min() {
        let result = Value::Integer(i32::MIN).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    // === Edge cases: Decimal → Integer truncation ===
    #[test]
    fn test_decimal_to_integer_truncates_positive() {
        let result = Value::Decimal(3.9).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(3));
    }

    #[test]
    fn test_decimal_to_integer_truncates_negative() {
        // Rust `as i32` truncates toward zero: -3.9 → -3
        let result = Value::Decimal(-3.9).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(-3));
    }

    #[test]
    fn test_decimal_to_integer_zero() {
        let result = Value::Decimal(0.0).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(0));
    }

    // === Edge cases: Integer → Decimal ===
    #[test]
    fn test_integer_to_decimal_negative() {
        let result = Value::Integer(-42).try_convert_to(ValueType::Decimal).unwrap();
        assert_value!(result, Decimal(-42.0));
    }

    #[test]
    fn test_integer_to_decimal_zero() {
        let result = Value::Integer(0).try_convert_to(ValueType::Decimal).unwrap();
        assert_value!(result, Decimal(0.0));
    }

    // === Edge cases: Integer/Decimal → String ===
    #[test]
    fn test_integer_to_string_negative() {
        let result = Value::Integer(-42).try_convert_to(ValueType::String).unwrap();
        assert_value!(result, String("-42"));
    }

    #[test]
    fn test_integer_to_string_zero() {
        let result = Value::Integer(0).try_convert_to(ValueType::String).unwrap();
        assert_value!(result, String("0"));
    }

    // === Edge cases: Bool → String ===
    #[test]
    fn test_bool_false_to_string() {
        let result = Value::Bool(false).try_convert_to(ValueType::String).unwrap();
        assert_value!(result, String("false"));
    }

    // === Edge cases: String → Bool rejects numeric strings ===
    #[test]
    fn test_string_one_to_bool_fails() {
        // "1" is not "true", should fail
        let result = Value::String("1".to_string()).try_convert_to(ValueType::Bool);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_zero_to_bool_fails() {
        // "0" is not "false", should fail
        let result = Value::String("0".to_string()).try_convert_to(ValueType::Bool);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_empty_to_bool_fails() {
        let result = Value::String("".to_string()).try_convert_to(ValueType::Bool);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_to_integer_negative() {
        let result = Value::String("-42".to_string()).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(-42));
    }

    #[test]
    fn test_string_to_decimal_negative() {
        let result = Value::String("-3.14".to_string()).try_convert_to(ValueType::Decimal).unwrap();
        assert_value!(result, Decimal(-3.14));
    }

    #[test]
    fn test_string_empty_to_integer_fails() {
        let result = Value::String("".to_string()).try_convert_to(ValueType::Integer);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_empty_to_decimal_fails() {
        let result = Value::String("".to_string()).try_convert_to(ValueType::Decimal);
        assert!(result.is_err());
    }

    // === Unsupported conversions ===
    // === Integer → Color (0..255 → grayscale) ===

    #[test]
    fn test_integer_to_color_zero_is_black() {
        let result = Value::Integer(0).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => { assert_eq!(c.r, 0.0); assert_eq!(c.g, 0.0); assert_eq!(c.b, 0.0); assert_eq!(c.a, 1.0); }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_integer_to_color_255_is_white() {
        let result = Value::Integer(255).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => { assert_eq!(c.r, 1.0); assert_eq!(c.g, 1.0); assert_eq!(c.b, 1.0); assert_eq!(c.a, 1.0); }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_integer_to_color_128_is_grey() {
        let result = Value::Integer(128).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => {
                let expected = 128.0 / 255.0;
                assert!((c.r - expected).abs() < 1e-6);
                assert!((c.g - expected).abs() < 1e-6);
                assert!((c.b - expected).abs() < 1e-6);
                assert_eq!(c.a, 1.0);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_integer_to_color_clamps_negative() {
        let result = Value::Integer(-50).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => { assert_eq!(c.r, 0.0); assert_eq!(c.g, 0.0); assert_eq!(c.b, 0.0); }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_integer_to_color_clamps_above_255() {
        let result = Value::Integer(999).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => { assert_eq!(c.r, 1.0); assert_eq!(c.g, 1.0); assert_eq!(c.b, 1.0); }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    // === Decimal → Color (0.0..1.0 → grayscale) ===

    #[test]
    fn test_decimal_to_color_zero_is_black() {
        let result = Value::Decimal(0.0).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => { assert_eq!(c.r, 0.0); assert_eq!(c.g, 0.0); assert_eq!(c.b, 0.0); assert_eq!(c.a, 1.0); }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_decimal_to_color_one_is_white() {
        let result = Value::Decimal(1.0).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => { assert_eq!(c.r, 1.0); assert_eq!(c.g, 1.0); assert_eq!(c.b, 1.0); assert_eq!(c.a, 1.0); }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_decimal_to_color_half_is_grey() {
        let result = Value::Decimal(0.5).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => {
                assert!((c.r - 0.5).abs() < 1e-6);
                assert!((c.g - 0.5).abs() < 1e-6);
                assert!((c.b - 0.5).abs() < 1e-6);
                assert_eq!(c.a, 1.0);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_decimal_to_color_clamps_negative() {
        let result = Value::Decimal(-0.5).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => { assert_eq!(c.r, 0.0); assert_eq!(c.g, 0.0); assert_eq!(c.b, 0.0); }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_decimal_to_color_clamps_above_one() {
        let result = Value::Decimal(2.5).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => { assert_eq!(c.r, 1.0); assert_eq!(c.g, 1.0); assert_eq!(c.b, 1.0); }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    // === Integer → DynamicImage (1x1 grayscale) ===

    #[test]
    fn test_integer_to_dynamic_image_zero() {
        let result = Value::Integer(0).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, .. } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                assert_eq!(pixel.0, [0, 0, 0, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[test]
    fn test_integer_to_dynamic_image_255() {
        let result = Value::Integer(255).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, .. } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                assert_eq!(pixel.0, [255, 255, 255, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[test]
    fn test_integer_to_dynamic_image_clamps() {
        let result = Value::Integer(999).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, .. } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                assert_eq!(pixel.0, [255, 255, 255, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    // === Decimal → DynamicImage (1x1 grayscale) ===

    #[test]
    fn test_decimal_to_dynamic_image_zero() {
        let result = Value::Decimal(0.0).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, .. } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                assert_eq!(pixel.0, [0, 0, 0, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[test]
    fn test_decimal_to_dynamic_image_one() {
        let result = Value::Decimal(1.0).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, .. } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                assert_eq!(pixel.0, [255, 255, 255, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[test]
    fn test_decimal_to_dynamic_image_half() {
        let result = Value::Decimal(0.5).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, .. } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                // 0.5 * 255 = 127
                assert_eq!(pixel.0[0], 127);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[test]
    fn test_decimal_to_dynamic_image_clamps_negative() {
        let result = Value::Decimal(-1.0).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, .. } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                assert_eq!(pixel.0, [0, 0, 0, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    // === Color → Bool ===

    #[test]
    fn test_color_to_bool_nonblack_is_true() {
        let c = Color::from_srgb_float(0.5, 0.0, 0.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_color_to_bool_black_is_false() {
        let c = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(false));
    }

    #[test]
    fn test_color_to_bool_black_with_alpha_is_false() {
        // Alpha doesn't affect truthiness — only RGB
        let c = Color::from_srgb_float(0.0, 0.0, 0.0, 0.5);
        let result = Value::Color(c).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(false));
    }

    #[test]
    fn test_color_to_bool_white_is_true() {
        let c = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    // === Color → Integer (luminance 0..255) ===

    #[test]
    fn test_color_to_integer_black() {
        let c = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(0));
    }

    #[test]
    fn test_color_to_integer_white() {
        let c = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(255));
    }

    #[test]
    fn test_color_to_integer_red() {
        // Luminance of pure red: 0.2126 * 1.0 * 255 ≈ 54
        let c = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::Integer).unwrap();
        match result {
            Value::Integer(v) => assert_eq!(v, 54),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[test]
    fn test_color_to_integer_green() {
        // Luminance of pure green: 0.7152 * 1.0 * 255 ≈ 182
        let c = Color::from_srgb_float(0.0, 1.0, 0.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::Integer).unwrap();
        match result {
            Value::Integer(v) => assert_eq!(v, 182),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    // === Color → Decimal (luminance 0.0..1.0) ===

    #[test]
    fn test_color_to_decimal_black() {
        let c = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::Decimal).unwrap();
        assert_value!(result, Decimal(0.0));
    }

    #[test]
    fn test_color_to_decimal_white() {
        let c = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::Decimal).unwrap();
        // 0.2126 + 0.7152 + 0.0722 = 1.0
        assert_value!(result, Decimal(1.0));
    }

    #[test]
    fn test_color_to_decimal_red() {
        let c = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::Decimal).unwrap();
        assert_value!(result, Decimal(0.2126));
    }

    // === Color → String ===

    #[test]
    fn test_color_to_string() {
        let c = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::String).unwrap();
        match result {
            Value::String(s) => {
                assert!(s.starts_with("rgba("));
                assert!(s.contains("0.5"));
                assert!(s.contains("0.3"));
                assert!(s.contains("0.7"));
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    // === Color → DynamicImage (1x1 solid color) ===

    #[test]
    fn test_color_to_dynamic_image() {
        let c = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, .. } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                assert_eq!(pixel.0[0], 255); // red
                assert_eq!(pixel.0[1], 0);   // green
                assert_eq!(pixel.0[2], 0);   // blue
                assert_eq!(pixel.0[3], 255); // alpha
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[test]
    fn test_color_to_dynamic_image_black() {
        let c = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let result = Value::Color(c).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, .. } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                assert_eq!(pixel.0, [0, 0, 0, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    // === String → Path ===

    #[test]
    fn test_string_to_path() {
        let result = Value::String("/test/file.txt".to_string()).try_convert_to(ValueType::Path).unwrap();
        match result {
            Value::Path(p) => assert_eq!(p, PathBuf::from("/test/file.txt")),
            other => panic!("Expected Path, got {:?}", other),
        }
    }

    #[test]
    fn test_string_to_path_empty() {
        let result = Value::String("".to_string()).try_convert_to(ValueType::Path).unwrap();
        match result {
            Value::Path(p) => assert_eq!(p, PathBuf::from("")),
            other => panic!("Expected Path, got {:?}", other),
        }
    }

    // === Still-unsupported conversions ===

    #[test]
    fn test_string_to_color_fails() {
        let result = Value::String("red".to_string()).try_convert_to(ValueType::Color);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_to_dynamic_image_fails() {
        let result = Value::String("img".to_string()).try_convert_to(ValueType::DynamicImage);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_to_bool_fails() {
        let result = Value::Path(PathBuf::from("/test")).try_convert_to(ValueType::Bool);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_to_integer_fails() {
        let result = Value::Path(PathBuf::from("/test")).try_convert_to(ValueType::Integer);
        assert!(result.is_err());
    }

    // === Bool → DynamicImage edge case: false produces black pixel ===
    #[test]
    fn test_bool_false_to_dynamic_image() {
        let result = Value::Bool(false).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, change_id: _ } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                assert_eq!(pixel.0, [0, 0, 0, 0]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[test]
    fn test_bool_true_to_dynamic_image_white() {
        let result = Value::Bool(true).try_convert_to(ValueType::DynamicImage).unwrap();
        match result {
            Value::DynamicImage { data, change_id: _ } => {
                let pixel = data.as_rgba8().unwrap().get_pixel(0, 0);
                assert_eq!(pixel.0, [255, 255, 255, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    // === Fingerprint tests ===
    #[test]
    fn test_fingerprint_same_value() {
        assert_eq!(Value::Integer(42).fingerprint(), Value::Integer(42).fingerprint());
    }

    #[test]
    fn test_fingerprint_different_values() {
        assert_ne!(Value::Integer(42).fingerprint(), Value::Integer(43).fingerprint());
    }

    #[test]
    fn test_fingerprint_different_types_same_number() {
        // Integer(1) and Decimal(1.0) should have different fingerprints (different discriminant)
        assert_ne!(Value::Integer(1).fingerprint(), Value::Decimal(1.0).fingerprint());
    }

    #[test]
    fn test_fingerprint_bool_values() {
        assert_ne!(Value::Bool(true).fingerprint(), Value::Bool(false).fingerprint());
    }

    #[test]
    fn test_fingerprint_strings() {
        assert_eq!(
            Value::String("hello".to_string()).fingerprint(),
            Value::String("hello".to_string()).fingerprint()
        );
        assert_ne!(
            Value::String("hello".to_string()).fingerprint(),
            Value::String("world".to_string()).fingerprint()
        );
    }

    // valid_conversions tests
    #[test]
    fn test_bool_valid_conversions() {
        let conversions = ValueType::Bool.valid_conversions();
        assert!(conversions.contains(&ValueType::Bool));
        assert!(conversions.contains(&ValueType::Integer));
        assert!(conversions.contains(&ValueType::Decimal));
        assert!(conversions.contains(&ValueType::String));
        assert!(conversions.contains(&ValueType::Trigger));
    }

    #[test]
    fn test_dynamic_image_valid_conversions() {
        let conversions = ValueType::DynamicImage.valid_conversions();
        assert!(conversions.contains(&ValueType::DynamicImage));
        assert!(conversions.contains(&ValueType::Trigger));
        assert!(!conversions.contains(&ValueType::Integer));
    }

    #[test]
    fn test_integer_valid_conversions() {
        let conversions = ValueType::Integer.valid_conversions();
        assert!(conversions.contains(&ValueType::Bool));
        assert!(conversions.contains(&ValueType::Integer));
        assert!(conversions.contains(&ValueType::Decimal));
        assert!(conversions.contains(&ValueType::String));
    }

    #[test]
    fn test_value_type_name() {
        assert_eq!(ValueType::Bool.value_name(), "bool");
        assert_eq!(ValueType::Integer.value_name(), "integer");
        assert_eq!(ValueType::Decimal.value_name(), "decimal");
        assert_eq!(ValueType::String.value_name(), "string");
        assert_eq!(ValueType::Color.value_name(), "color");
        assert_eq!(ValueType::DynamicImage.value_name(), "image");
        assert_eq!(ValueType::Path.value_name(), "path");
    }
}
