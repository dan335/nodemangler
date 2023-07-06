use crate::color::Color;

impl Color {
    // rgba linear to srgba
    pub fn from_rgb_linear(red: f32, green: f32, blue: f32, alpha: f32) -> Color {
        Color {
            r: linear_to_nonlinear_srgb(red),
            g: linear_to_nonlinear_srgb(green),
            b: linear_to_nonlinear_srgb(blue),
            a: alpha,
        }
    }

    // srgba to rgba linear
    pub fn to_rgb_linear(&self) -> (f32, f32, f32, f32) {
        (
            nonlinear_to_linear_rgb(self.r),
            nonlinear_to_linear_rgb(self.g),
            nonlinear_to_linear_rgb(self.b),
            self.a,
        )
    }
}

#[inline]
pub fn nonlinear_to_linear_rgb(n: f32) -> f32 {
    if n <= 0.0 {
        return n;
    }
    if n <= 0.04045 {
        n / 12.92 // linear falloff in dark values
    } else {
        ((n + 0.055) / 1.055).powf(2.4) // gamma curve in other area
    }
}

#[inline]
pub fn linear_to_nonlinear_srgb(n: f32) -> f32 {
    if n <= 0.0 {
        return n;
    }

    if n <= 0.0031308 {
        n * 12.92 // linear falloff in dark values
    } else {
        (1.055 * n.powf(1.0 / 2.4)) - 0.055 // gamma curve in other area
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn text_to_from_rgb_linear() {
        let color = Color::from_rgb_linear(0.75, 0.5, 0.25, 1.0);
        let (r, g, b, a) = color.to_rgb_linear();
        let color2 = Color::from_rgb_linear(r, g, b, a);
        assert_eq!(color, color2);
    }
}