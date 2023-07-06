use crate::color::Color;

static RGB2YUV_COEFFS: [f32; 5] = [0.299, 0.587, 0.114, 0.492, 0.877];
static YUV2RGB_COEFFS: [f32; 4] = [2.032, -0.395, -0.581, 1.14];

impl Color {
    pub fn from_yuv(y: f32, u: f32, v: f32, a: f32) -> Color {
        let r = y + YUV2RGB_COEFFS[3] * v;
        let g = y + YUV2RGB_COEFFS[1] * u + YUV2RGB_COEFFS[2] * v;
        let b = y + YUV2RGB_COEFFS[0] * u;
        Color { r, g, b, a }
    }

    pub fn to_yuv(&self) -> (f32, f32, f32, f32) {
        let y = RGB2YUV_COEFFS[0] * self.r + RGB2YUV_COEFFS[1] * self.g + RGB2YUV_COEFFS[2] * self.b;
        let u = RGB2YUV_COEFFS[3] * (self.b - y);
        let v = RGB2YUV_COEFFS[4] * (self.r - y);
        (y, u, v, self.a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn text_to_from_yuv() {
        let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
        let (y, u, v, a) = color.to_yuv();
        let color2 = Color::from_yuv(y, u, v, a);
        
        assert!(
            (color.r - color2.r).abs() < EPSILON,
            "Red channel mismatch: {} vs {}",
            color.r,
            color2.r
        );
        assert!(
            (color.g - color2.g).abs() < EPSILON,
            "Green channel mismatch: {} vs {}",
            color.g,
            color2.g
        );
        assert!(
            (color.b - color2.b).abs() < EPSILON,
            "Blue channel mismatch: {} vs {}",
            color.b,
            color2.b
        );
        assert!(
            (color.a - color2.a).abs() < EPSILON,
            "Alpha channel mismatch: {} vs {}",
            color.a,
            color2.a
        );
    }
}