//! Channel-agnostic f32 image buffer for internal graph processing.
//!
//! [`FloatImage`] stores pixel data as interleaved f32 values with a dynamic
//! channel count (1–4). All image operations in the node graph work on this
//! type directly, avoiding unnecessary format conversions. Conversion to/from
//! the `image` crate's [`DynamicImage`] happens only at I/O boundaries
//! (file load/save, clipboard, GUI display).

use image::{DynamicImage, RgbaImage};

/// A channel-agnostic f32 image buffer.
///
/// Pixel data is stored as a flat `Vec<f32>` in row-major, channel-interleaved
/// order. For a 2×2 image with 3 channels the layout is:
/// `[r00, g00, b00, r10, g10, b10, r01, g01, b01, r11, g11, b11]`.
///
/// Channel count is fixed at construction and must be 1–4:
/// - 1 channel: grayscale (noise, height maps, masks)
/// - 2 channels: grayscale + alpha
/// - 3 channels: RGB color
/// - 4 channels: RGBA color
#[derive(Clone, Debug)]
pub struct FloatImage {
    /// Flat pixel buffer: length = width * height * channels.
    data: Vec<f32>,
    /// Image width in pixels.
    width: u32,
    /// Image height in pixels.
    height: u32,
    /// Number of channels per pixel (1–4).
    channels: u32,
}

impl FloatImage {
    /// Creates a new zero-filled image with the given dimensions and channel count.
    ///
    /// # Panics
    /// Panics if `channels` is not in the range 1..=4.
    pub fn new(width: u32, height: u32, channels: u32) -> Self {
        assert!((1..=4).contains(&channels), "channels must be 1–4, got {}", channels);
        Self {
            data: vec![0.0; (width * height * channels) as usize],
            width,
            height,
            channels,
        }
    }

    /// Creates an image filled with a single pixel value repeated across all pixels.
    ///
    /// `pixel` must have exactly `channels` elements.
    ///
    /// # Panics
    /// Panics if `channels` is not 1–4 or if `pixel.len() != channels`.
    pub fn from_pixel(width: u32, height: u32, channels: u32, pixel: &[f32]) -> Self {
        assert!((1..=4).contains(&channels), "channels must be 1–4, got {}", channels);
        assert_eq!(pixel.len(), channels as usize, "pixel length must match channels");
        let total = (width * height) as usize;
        let mut data = Vec::with_capacity(total * channels as usize);
        for _ in 0..total {
            data.extend_from_slice(pixel);
        }
        Self { data, width, height, channels }
    }

    /// Creates a FloatImage from raw data. Returns `None` if the data length
    /// does not match `width * height * channels`.
    pub fn from_raw(width: u32, height: u32, channels: u32, data: Vec<f32>) -> Option<Self> {
        if data.len() != (width * height * channels) as usize {
            return None;
        }
        Some(Self { data, width, height, channels })
    }

    /// Converts a `DynamicImage` into a `FloatImage`, preserving the channel count.
    ///
    /// Channel mapping:
    /// - Luma8/Luma16 → 1 channel
    /// - LumaA8/LumaA16 → 2 channels
    /// - Rgb8/Rgb16/Rgb32F → 3 channels
    /// - Rgba8/Rgba16/Rgba32F → 4 channels
    pub fn from_dynamic(img: &DynamicImage) -> Self {
        match img {
            // 1-channel (grayscale)
            DynamicImage::ImageLuma8(buf) => {
                let (w, h) = buf.dimensions();
                let data: Vec<f32> = buf.as_raw().iter().map(|&v| v as f32 / 255.0).collect();
                Self { data, width: w, height: h, channels: 1 }
            }
            DynamicImage::ImageLuma16(buf) => {
                let (w, h) = buf.dimensions();
                let data: Vec<f32> = buf.as_raw().iter().map(|&v| v as f32 / 65535.0).collect();
                Self { data, width: w, height: h, channels: 1 }
            }
            // 2-channel (grayscale + alpha)
            DynamicImage::ImageLumaA8(buf) => {
                let (w, h) = buf.dimensions();
                let data: Vec<f32> = buf.as_raw().iter().map(|&v| v as f32 / 255.0).collect();
                Self { data, width: w, height: h, channels: 2 }
            }
            DynamicImage::ImageLumaA16(buf) => {
                let (w, h) = buf.dimensions();
                let data: Vec<f32> = buf.as_raw().iter().map(|&v| v as f32 / 65535.0).collect();
                Self { data, width: w, height: h, channels: 2 }
            }
            // 3-channel (RGB)
            DynamicImage::ImageRgb8(buf) => {
                let (w, h) = buf.dimensions();
                let data: Vec<f32> = buf.as_raw().iter().map(|&v| v as f32 / 255.0).collect();
                Self { data, width: w, height: h, channels: 3 }
            }
            DynamicImage::ImageRgb16(buf) => {
                let (w, h) = buf.dimensions();
                let data: Vec<f32> = buf.as_raw().iter().map(|&v| v as f32 / 65535.0).collect();
                Self { data, width: w, height: h, channels: 3 }
            }
            DynamicImage::ImageRgb32F(buf) => {
                let (w, h) = buf.dimensions();
                let data: Vec<f32> = buf.as_raw().clone();
                Self { data, width: w, height: h, channels: 3 }
            }
            // 4-channel (RGBA)
            DynamicImage::ImageRgba8(buf) => {
                let (w, h) = buf.dimensions();
                let data: Vec<f32> = buf.as_raw().iter().map(|&v| v as f32 / 255.0).collect();
                Self { data, width: w, height: h, channels: 4 }
            }
            DynamicImage::ImageRgba16(buf) => {
                let (w, h) = buf.dimensions();
                let data: Vec<f32> = buf.as_raw().iter().map(|&v| v as f32 / 65535.0).collect();
                Self { data, width: w, height: h, channels: 4 }
            }
            DynamicImage::ImageRgba32F(buf) => {
                let (w, h) = buf.dimensions();
                let data: Vec<f32> = buf.as_raw().clone();
                Self { data, width: w, height: h, channels: 4 }
            }
            // Fallback for any future variants
            _ => {
                let rgba = img.to_rgba32f();
                let (w, h) = rgba.dimensions();
                let data: Vec<f32> = rgba.into_raw();
                Self { data, width: w, height: h, channels: 4 }
            }
        }
    }

    /// Converts this FloatImage to a `DynamicImage` for GUI display or file output.
    ///
    /// Channel mapping:
    /// - 1ch → ImageLuma16 (best lossless grayscale available in DynamicImage)
    /// - 2ch → ImageLumaA16
    /// - 3ch → ImageRgb32F
    /// - 4ch → ImageRgba32F
    pub fn to_dynamic(&self) -> DynamicImage {
        match self.channels {
            1 => {
                let u16_data: Vec<u16> = self.data.iter()
                    .map(|&v| (v.clamp(0.0, 1.0) * 65535.0) as u16)
                    .collect();
                let buf = image::ImageBuffer::from_raw(self.width, self.height, u16_data)
                    .expect("FloatImage data length mismatch");
                DynamicImage::ImageLuma16(buf)
            }
            2 => {
                let u16_data: Vec<u16> = self.data.iter()
                    .map(|&v| (v.clamp(0.0, 1.0) * 65535.0) as u16)
                    .collect();
                let buf = image::ImageBuffer::from_raw(self.width, self.height, u16_data)
                    .expect("FloatImage data length mismatch");
                DynamicImage::ImageLumaA16(buf)
            }
            3 => {
                let buf = image::ImageBuffer::from_raw(self.width, self.height, self.data.clone())
                    .expect("FloatImage data length mismatch");
                DynamicImage::ImageRgb32F(buf)
            }
            4 => {
                let buf = image::ImageBuffer::from_raw(self.width, self.height, self.data.clone())
                    .expect("FloatImage data length mismatch");
                DynamicImage::ImageRgba32F(buf)
            }
            _ => unreachable!("channels must be 1–4"),
        }
    }

    /// Converts this FloatImage to an RGBA8 image for thumbnails, clipboard, and GUI display.
    ///
    /// Channel expansion:
    /// - 1ch: R=G=B=value, A=255
    /// - 2ch: R=G=B=gray, A=alpha
    /// - 3ch: R=r, G=g, B=b, A=255
    /// - 4ch: R=r, G=g, B=b, A=a
    pub fn to_rgba8(&self) -> RgbaImage {
        let mut out = RgbaImage::new(self.width, self.height);
        let ch = self.channels as usize;

        for (i, pixel) in out.pixels_mut().enumerate() {
            let offset = i * ch;
            let src = &self.data[offset..offset + ch];
            let rgba = match ch {
                1 => {
                    let v = (src[0].clamp(0.0, 1.0) * 255.0) as u8;
                    [v, v, v, 255]
                }
                2 => {
                    let v = (src[0].clamp(0.0, 1.0) * 255.0) as u8;
                    let a = (src[1].clamp(0.0, 1.0) * 255.0) as u8;
                    [v, v, v, a]
                }
                3 => {
                    [
                        (src[0].clamp(0.0, 1.0) * 255.0) as u8,
                        (src[1].clamp(0.0, 1.0) * 255.0) as u8,
                        (src[2].clamp(0.0, 1.0) * 255.0) as u8,
                        255,
                    ]
                }
                4 => {
                    [
                        (src[0].clamp(0.0, 1.0) * 255.0) as u8,
                        (src[1].clamp(0.0, 1.0) * 255.0) as u8,
                        (src[2].clamp(0.0, 1.0) * 255.0) as u8,
                        (src[3].clamp(0.0, 1.0) * 255.0) as u8,
                    ]
                }
                _ => unreachable!(),
            };
            *pixel = image::Rgba(rgba);
        }
        out
    }

    /// Image width in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Number of channels per pixel (1–4).
    #[inline]
    pub fn channels(&self) -> u32 {
        self.channels
    }

    /// Returns (width, height).
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the raw f32 pixel data as a slice.
    #[inline]
    pub fn as_raw(&self) -> &[f32] {
        &self.data
    }

    /// Returns the raw f32 pixel data as a mutable slice.
    #[inline]
    pub fn as_raw_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// Consume the image and return its raw pixel buffer. Used by callers
    /// that want to reclaim the `Vec<f32>` allocation (e.g. the video decoder
    /// cache reuses these across successive frames of a clip).
    #[inline]
    pub fn into_data(self) -> Vec<f32> {
        self.data
    }

    /// Returns the pixel at (x, y) as a slice of `channels` f32 values.
    ///
    /// # Panics
    /// Panics if (x, y) is out of bounds.
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> &[f32] {
        let idx = self.pixel_offset(x, y);
        &self.data[idx..idx + self.channels as usize]
    }

    /// Returns a mutable reference to the pixel at (x, y).
    ///
    /// # Panics
    /// Panics if (x, y) is out of bounds.
    #[inline]
    pub fn get_pixel_mut(&mut self, x: u32, y: u32) -> &mut [f32] {
        let idx = self.pixel_offset(x, y);
        let ch = self.channels as usize;
        &mut self.data[idx..idx + ch]
    }

    /// Writes pixel values at (x, y). `pixel` must have exactly `channels` elements.
    ///
    /// # Panics
    /// Panics if (x, y) is out of bounds or `pixel.len() != channels`.
    #[inline]
    pub fn put_pixel(&mut self, x: u32, y: u32, pixel: &[f32]) {
        debug_assert_eq!(pixel.len(), self.channels as usize);
        let idx = self.pixel_offset(x, y);
        let ch = self.channels as usize;
        self.data[idx..idx + ch].copy_from_slice(pixel);
    }

    /// Returns an iterator over all pixels, each as a slice of `channels` f32 values.
    pub fn pixels(&self) -> impl Iterator<Item = &[f32]> {
        self.data.chunks_exact(self.channels as usize)
    }

    /// Returns a mutable iterator over all pixels.
    pub fn pixels_mut(&mut self) -> impl Iterator<Item = &mut [f32]> {
        self.data.chunks_exact_mut(self.channels as usize)
    }

    /// Returns an iterator yielding `(x, y, pixel_slice)` for every pixel.
    pub fn enumerate_pixels(&self) -> impl Iterator<Item = (u32, u32, &[f32])> {
        let w = self.width;
        let ch = self.channels as usize;
        self.data.chunks_exact(ch).enumerate().map(move |(i, px)| {
            let x = (i as u32) % w;
            let y = (i as u32) / w;
            (x, y, px)
        })
    }

    /// Returns a mutable iterator yielding `(x, y, pixel_slice)` for every pixel.
    pub fn enumerate_pixels_mut(&mut self) -> impl Iterator<Item = (u32, u32, &mut [f32])> {
        let w = self.width;
        let ch = self.channels as usize;
        self.data.chunks_exact_mut(ch).enumerate().map(move |(i, px)| {
            let x = (i as u32) % w;
            let y = (i as u32) / w;
            (x, y, px)
        })
    }

    /// Samples the image at fractional coordinates using bilinear interpolation.
    ///
    /// Coordinates outside the image are clamped to the nearest edge pixel.
    /// Writes `channels` values into `out`. Returns all zeros if the image has
    /// zero dimensions.
    ///
    /// # Panics
    /// Panics if `out.len() < channels`.
    pub fn bilinear_sample(&self, x: f32, y: f32, out: &mut [f32]) {
        let ch = self.channels as usize;
        debug_assert!(out.len() >= ch);

        let (w, h) = (self.width, self.height);
        if w == 0 || h == 0 {
            for v in &mut out[..ch] { *v = 0.0; }
            return;
        }

        // Compute the four surrounding integer pixel coordinates, clamped to image bounds
        let x0 = (x.floor() as i32).clamp(0, w as i32 - 1) as u32;
        let y0 = (y.floor() as i32).clamp(0, h as i32 - 1) as u32;
        let x1 = (x0 + 1).min(w - 1);
        let y1 = (y0 + 1).min(h - 1);

        // Fractional parts determine the interpolation weights
        let fx = x - x.floor();
        let fy = y - y.floor();

        // Sample the four neighboring pixels
        let p00 = self.get_pixel(x0, y0);
        let p10 = self.get_pixel(x1, y0);
        let p01 = self.get_pixel(x0, y1);
        let p11 = self.get_pixel(x1, y1);

        // Weighted average across all channels
        for i in 0..ch {
            out[i] = p00[i] * (1.0 - fx) * (1.0 - fy)
                + p10[i] * fx * (1.0 - fy)
                + p01[i] * (1.0 - fx) * fy
                + p11[i] * fx * fy;
        }
    }

    /// Resizes the image to the given dimensions using bilinear interpolation.
    /// Preserves the channel count.
    pub fn resize(&self, new_w: u32, new_h: u32) -> Self {
        if new_w == 0 || new_h == 0 {
            return Self::new(new_w, new_h, self.channels);
        }

        let ch = self.channels as usize;
        let mut result = Self::new(new_w, new_h, self.channels);
        let mut sample_buf = vec![0.0f32; ch];

        // Scale factors map output coordinates to source coordinates
        let x_scale = if new_w > 1 { (self.width as f32 - 1.0) / (new_w as f32 - 1.0) } else { 0.0 };
        let y_scale = if new_h > 1 { (self.height as f32 - 1.0) / (new_h as f32 - 1.0) } else { 0.0 };

        for y in 0..new_h {
            for x in 0..new_w {
                let sx = x as f32 * x_scale;
                let sy = y as f32 * y_scale;
                self.bilinear_sample(sx, sy, &mut sample_buf);
                result.put_pixel(x, y, &sample_buf);
            }
        }
        result
    }

    /// Resizes the image to fit within `max_w × max_h` while preserving aspect ratio.
    ///
    /// The result will be at most `max_w` wide and `max_h` tall, but may be smaller
    /// on one axis to maintain the original proportions. Matches the behavior of
    /// `DynamicImage::resize()` from the `image` crate.
    pub fn resize_fit(&self, max_w: u32, max_h: u32) -> Self {
        if self.width == 0 || self.height == 0 || max_w == 0 || max_h == 0 {
            return Self::new(max_w.min(self.width), max_h.min(self.height), self.channels);
        }

        // Compute the scale factor that fits the image within the bounding box
        let scale = (max_w as f32 / self.width as f32)
            .min(max_h as f32 / self.height as f32);
        let new_w = ((self.width as f32 * scale).round() as u32).max(1);
        let new_h = ((self.height as f32 * scale).round() as u32).max(1);

        self.resize(new_w, new_h)
    }

    /// Returns the byte offset into `self.data` for pixel (x, y).
    #[inline]
    fn pixel_offset(&self, x: u32, y: u32) -> usize {
        ((y * self.width + x) * self.channels) as usize
    }
}

#[cfg(test)]
#[path = "float_image_tests.rs"]
mod tests;
