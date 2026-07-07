//! Procedural environment lighting — pure CPU, no GL.
//!
//! The 3D viewer lights materials from a fixed procedural sky (a three-colour
//! vertical gradient) instead of an HDRI asset. Because that sky is
//! *yaw-symmetric* (rotationally symmetric around +Y, with the sun deliberately
//! excluded, see below), the image-based-lighting (IBL) convolutions collapse
//! from full cubemaps to tiny 1-D/2-D look-up tables:
//!
//! - **Diffuse irradiance** depends only on the normal's `N.y` → a `64×1` LUT.
//! - **Prefiltered specular** depends only on `(R.y, roughness)` → a `64×64` LUT.
//!
//! Both LUTs are prefiltered *once* on the CPU at renderer init (no FBOs, no
//! render-to-texture dance with egui_glow) and uploaded as `RGBA32F` textures.
//! Everything here is plain Rust and therefore unit-testable.
//!
//! The **sun is excluded** from these LUTs on purpose: the analytic directional
//! light in the fragment shader already contributes the sun's radiance, so
//! folding it into the IBL as well would double-count it. The sun *is* drawn in
//! the visible skybox background (see `gl_renderer.rs`), just not baked here.
//!
//! The sky colours are **fixed constants, not theme-derived** — the lighting of
//! a material must not shift when the user switches the UI theme.

use glam::Vec3;

/// Sky colour at the zenith (straight up, `dir.y = +1`). A cool blue.
pub const ZENITH: Vec3 = Vec3::new(0.35, 0.45, 0.65);
/// Sky colour at the horizon (`dir.y = 0`). A warm, slightly desaturated tone
/// where the sky meets the ground.
pub const HORIZON: Vec3 = Vec3::new(0.60, 0.55, 0.50);
/// "Ground" colour seen looking down (`dir.y = -1`). A dim warm brown, standing
/// in for bounce light off an unseen floor.
pub const GROUND: Vec3 = Vec3::new(0.22, 0.18, 0.15);

/// Width (in texels) of the diffuse irradiance LUT; the single row is indexed by
/// `N.y` remapped to `[0,1]`. Documented as a const so the GL upload and the
/// tests agree on the size.
pub const IRRADIANCE_SIZE: usize = 64;

/// Side length (in texels) of the square specular LUT: x-axis is `R.y` remapped
/// to `[0,1]`, y-axis is `roughness` in `[0,1]`.
pub const SPECULAR_SIZE: usize = 64;

/// GLSL-matching smoothstep: `clamp`ed Hermite interpolation between `edge0` and
/// `edge1`. Kept identical to GLSL's built-in `smoothstep` so the CPU sky and the
/// GPU skybox (which uses the built-in) evaluate the same gradient.
#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Radiance of the procedural sky in a given world-space direction.
///
/// Above the horizon the colour blends `HORIZON → ZENITH` as `dir.y` rises to
/// `+1`; below it blends `HORIZON → GROUND` as `dir.y` falls to `-1`. Both sides
/// meet at exactly `HORIZON` when `dir.y = 0`, and the `smoothstep` shaping gives
/// a soft band across the horizon so neighbouring directions never jump in
/// colour. **No sun term** — see the module docs.
pub fn sky_radiance(dir: Vec3) -> Vec3 {
    // Normalise defensively; callers pass unit vectors but the LUT builders
    // construct directions arithmetically.
    let y = dir.normalize_or(Vec3::Y).y;
    if y >= 0.0 {
        let t = smoothstep(0.0, 1.0, y);
        HORIZON.lerp(ZENITH, t)
    } else {
        let t = smoothstep(0.0, 1.0, -y);
        HORIZON.lerp(GROUND, t)
    }
}

/// Build an orthonormal basis `(tangent, bitangent)` for a unit normal `n`.
///
/// Picks a reference up vector not parallel to `n` (falls back to +X near the
/// poles where +Y would be degenerate), then Gram-Schmidt-free cross products.
#[inline]
fn orthonormal_basis(n: Vec3) -> (Vec3, Vec3) {
    let up = if n.y.abs() < 0.999 { Vec3::Y } else { Vec3::X };
    let tangent = up.cross(n).normalize();
    let bitangent = n.cross(tangent);
    (tangent, bitangent)
}

/// Van der Corput radical inverse in base 2 (bit-reversal), for the Hammersley
/// low-discrepancy sequence used by the specular importance sampling.
#[inline]
fn radical_inverse_vdc(mut bits: u32) -> f32 {
    bits = (bits << 16) | (bits >> 16);
    bits = ((bits & 0x5555_5555) << 1) | ((bits & 0xAAAA_AAAA) >> 1);
    bits = ((bits & 0x3333_3333) << 2) | ((bits & 0xCCCC_CCCC) >> 2);
    bits = ((bits & 0x0F0F_0F0F) << 4) | ((bits & 0xF0F0_F0F0) >> 4);
    bits = ((bits & 0x00FF_00FF) << 8) | ((bits & 0xFF00_FF00) >> 8);
    (bits as f32) * 2.328_306_4e-10 // 1 / 2^32
}

/// The i-th of `n` Hammersley sample points on the unit square `[0,1)²`.
#[inline]
fn hammersley(i: u32, n: u32) -> (f32, f32) {
    (i as f32 / n as f32, radical_inverse_vdc(i))
}

/// GGX/Trowbridge-Reitz importance sample: map a uniform pair `(u1, u2)` to a
/// microfacet half-vector around normal `n` for the given `roughness`. At
/// `roughness = 0` every sample collapses onto `n` (perfect mirror).
#[inline]
fn importance_sample_ggx(u1: f32, u2: f32, n: Vec3, roughness: f32) -> Vec3 {
    let a = roughness * roughness;
    let phi = 2.0 * std::f32::consts::PI * u1;
    // Sample the GGX distribution of half-vector elevation.
    let cos_theta = ((1.0 - u2) / (1.0 + (a * a - 1.0) * u2)).sqrt();
    let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
    // Half-vector in the normal's tangent space (z = normal).
    let h_tangent = Vec3::new(phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta);
    let (tangent, bitangent) = orthonormal_basis(n);
    (tangent * h_tangent.x + bitangent * h_tangent.y + n * h_tangent.z).normalize()
}

/// Remap a texel index (with its half-texel centre offset) across `size` texels
/// to the signed axis value in `[-1, 1]` — the inverse of the shader's
/// `axis * 0.5 + 0.5` texture-coordinate mapping.
#[inline]
fn axis_from_index(index: usize, size: usize) -> f32 {
    (index as f32 + 0.5) / size as f32 * 2.0 - 1.0
}

/// Build the diffuse irradiance LUT: `IRRADIANCE_SIZE × 1` interleaved `RGBA32F`
/// texels (alpha = 1.0).
///
/// Each texel is the cosine-weighted hemisphere integral of `sky_radiance`
/// around a normal whose `y` component is the texel's `N.y` (the horizontal
/// component points along +X — any yaw works by symmetry). With cosine-weighted
/// importance sampling the Monte-Carlo estimator is simply the average of
/// `sky_radiance` over the samples, so a uniform sky reproduces its own colour.
pub fn build_irradiance_lut() -> Vec<f32> {
    // Stratified sample grid over the hemisphere (elevation × azimuth).
    const N_THETA: u32 = 32;
    const N_PHI: u32 = 64;
    let inv_count = 1.0 / (N_THETA * N_PHI) as f32;

    let mut data = Vec::with_capacity(IRRADIANCE_SIZE * 4);
    for i in 0..IRRADIANCE_SIZE {
        let y = axis_from_index(i, IRRADIANCE_SIZE);
        // Normal for this bucket: yaw-symmetric, so put the horizontal part on +X.
        let n = Vec3::new((1.0 - y * y).max(0.0).sqrt(), y, 0.0).normalize_or(Vec3::Y);
        let (tangent, bitangent) = orthonormal_basis(n);

        let mut sum = Vec3::ZERO;
        for it in 0..N_THETA {
            for ip in 0..N_PHI {
                // Stratified sample centres on the unit square.
                let u1 = (it as f32 + 0.5) / N_THETA as f32;
                let u2 = (ip as f32 + 0.5) / N_PHI as f32;
                // Cosine-weighted hemisphere sample in tangent space (z = normal).
                let r = u1.sqrt();
                let phi = 2.0 * std::f32::consts::PI * u2;
                let x = r * phi.cos();
                let yy = r * phi.sin();
                let z = (1.0 - u1).max(0.0).sqrt();
                let dir = (tangent * x + bitangent * yy + n * z).normalize();
                sum += sky_radiance(dir);
            }
        }
        let irradiance = sum * inv_count;
        data.extend_from_slice(&[irradiance.x, irradiance.y, irradiance.z, 1.0]);
    }
    data
}

/// Build the prefiltered specular LUT: `SPECULAR_SIZE × SPECULAR_SIZE`
/// interleaved `RGBA32F` texels (alpha = 1.0), row-major with row 0 = the
/// smallest roughness.
///
/// x-axis is the reflection vector's `R.y` (remapped to texture coords), y-axis
/// is `roughness`. Each texel is a GGX importance-sampled convolution of
/// `sky_radiance` around the reflection direction (assuming `N = V = R`), the
/// standard split-sum prefilter. `SAMPLES` half-vectors per texel.
pub fn build_specular_lut() -> Vec<f32> {
    const SAMPLES: u32 = 128;

    let mut data = Vec::with_capacity(SPECULAR_SIZE * SPECULAR_SIZE * 4);
    // Rows: roughness 0..1 (endpoints inclusive so the mirror/rough extremes are
    // representable exactly). Row 0 is roughness 0 → sampled at texcoord.y = 0.
    for row in 0..SPECULAR_SIZE {
        let roughness = row as f32 / (SPECULAR_SIZE as f32 - 1.0);
        for col in 0..SPECULAR_SIZE {
            let ry = axis_from_index(col, SPECULAR_SIZE);
            // Reflection direction for this bucket (yaw-symmetric → horizontal on +X).
            let r = Vec3::new((1.0 - ry * ry).max(0.0).sqrt(), ry, 0.0).normalize_or(Vec3::Y);
            // Split-sum prefilter approximation: N = V = R.
            let (n, v) = (r, r);

            let mut color = Vec3::ZERO;
            let mut total_weight = 0.0;
            for s in 0..SAMPLES {
                let (u1, u2) = hammersley(s, SAMPLES);
                let h = importance_sample_ggx(u1, u2, n, roughness);
                // Reflect the view vector about the sampled half-vector.
                let l = (h * (2.0 * v.dot(h)) - v).normalize();
                let ndotl = n.dot(l).max(0.0);
                if ndotl > 0.0 {
                    color += sky_radiance(l) * ndotl;
                    total_weight += ndotl;
                }
            }
            if total_weight > 0.0 {
                color /= total_weight;
            }
            data.extend_from_slice(&[color.x, color.y, color.z, 1.0]);
        }
    }
    data
}

#[cfg(test)]
#[path = "environment_tests.rs"]
mod tests;
