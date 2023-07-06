use super::Color;

impl Color {

    pub fn blend_cmyk(a: Color, b: Color, amount: f32) -> Color {
        let la = a.to_cmyk();
        let lb = b.to_cmyk();

        Color::from_cmyk(
            lerp(la.0, lb.0, amount),
            lerp(la.1, lb.1, amount),
            lerp(la.2, lb.2, amount),
            lerp(la.3, lb.3, amount),
            lerp(la.4, lb.4, amount),
        )
    }

    pub fn blend_hsl(a: Color, b: Color, amount: f32) -> Color {
        let la = a.to_hsl();
        let lb = b.to_hsl();

        Color::from_hsl(
            lerp(la.0, lb.0, amount),
            lerp(la.1, lb.1, amount),
            lerp(la.2, lb.2, amount),
            lerp(la.3, lb.3, amount),
        )
    }

    pub fn blend_hsv(a: Color, b: Color, amount: f32) -> Color {
        let la = a.to_hsv();
        let lb = b.to_hsv();

        Color::from_hsv(
            lerp(la.0, lb.0, amount),
            lerp(la.1, lb.1, amount),
            lerp(la.2, lb.2, amount),
            lerp(la.3, lb.3, amount),
        )
    }

    pub fn blend_lab(a: Color, b: Color, amount: f32) -> Color {
        let la = a.to_lab();
        let lb = b.to_lab();

        Color::from_lab(
            lerp(la.0, lb.0, amount),
            lerp(la.1, lb.1, amount),
            lerp(la.2, lb.2, amount),
            lerp(la.3, lb.3, amount),
        )
    }

    pub fn blend_lch(a: Color, b: Color, amount: f32) -> Color {
        let la = a.to_lch();
        let lb = b.to_lch();

        Color::from_lch(
            lerp(la.0, lb.0, amount),
            lerp(la.1, lb.1, amount),
            lerp(la.2, lb.2, amount),
            lerp(la.3, lb.3, amount),
        )
    }

    // convert to rgb linear.  lerp.  convert back to srgb.
    pub fn blend_linear(a: Color, b: Color, amount: f32) -> Color {
        let la = a.to_rgb_linear();
        let lb = b.to_rgb_linear();

        Color::from_rgb_linear(
            lerp(la.0, lb.0, amount),
            lerp(la.1, lb.1, amount),
            lerp(la.2, lb.2, amount),
            lerp(la.3, lb.3, amount),
        )
    }

    pub fn blend_srgb(a: Color, b: Color, amount: f32) -> Color {
        Color::from_srgb_float(
            lerp(a.r, b.r, amount),
            lerp(a.g, b.g, amount),
            lerp(a.b, b.b, amount),
            lerp(a.a, b.a, amount),
        )
    }

    pub fn blend_xyz(a: Color, b: Color, amount: f32) -> Color {
        let la = a.to_xyz();
        let lb = b.to_xyz();

        Color::from_xyz(
            lerp(la.0, lb.0, amount),
            lerp(la.1, lb.1, amount),
            lerp(la.2, lb.2, amount),
            lerp(la.3, lb.3, amount),
        )
    }

    pub fn blend_yuv(a: Color, b: Color, amount: f32) -> Color {
        let la = a.to_yuv();
        let lb = b.to_yuv();

        Color::from_yuv(
            lerp(la.0, lb.0, amount),
            lerp(la.1, lb.1, amount),
            lerp(la.2, lb.2, amount),
            lerp(la.3, lb.3, amount),
        )
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}