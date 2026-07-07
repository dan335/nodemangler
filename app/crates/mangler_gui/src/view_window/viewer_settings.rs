//! Per-leaf, in-memory settings for the 3D material viewer (light + camera).
//!
//! These are owned by `Preview3dPanel` and snapshotted into a `RenderParams`
//! each frame. The struct is intentionally small in Phase 1 and grows in later
//! phases (geometry resolution, environment intensity, tone map, etc.).
//!
//! TODO(persistence): per-leaf viewer settings are in-memory only for now.
//! They are deliberately *not* written into the persisted layout config,
//! matching the existing per-leaf camera behavior.

use glam::Vec3;

/// Tone-mapping operator applied to the HDR render (and the skybox) before the
/// separate `pow(1/2.2)` gamma step. The integer values are wired straight to
/// the `u_tone_map` shader uniform via [`ToneMap::to_int`] — keep them in sync
/// with the `apply_tone_map()` branches in `TONE_MAP_GLSL` (see `gl_renderer`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToneMap {
    /// No compression — clamp to [0,1] only. Blows out HDR highlights.
    None,
    /// Reinhard `c / (c + 1)` (the viewer's original tone map).
    Reinhard,
    /// ACES filmic approximation (Narkowicz 2015 fit). The default.
    Aces,
    /// Hable/Uncharted 2 filmic curve with a linear white point.
    Filmic,
}

impl Default for ToneMap {
    fn default() -> Self {
        // ACES gives the most film-like highlight roll-off; matches the plan.
        ToneMap::Aces
    }
}

impl ToneMap {
    /// All operators, ordered for the UI combo box.
    pub const ALL: [ToneMap; 4] = [
        ToneMap::None,
        ToneMap::Reinhard,
        ToneMap::Aces,
        ToneMap::Filmic,
    ];

    /// Human-readable name for the tone-map combo box.
    pub fn label(&self) -> &'static str {
        match self {
            ToneMap::None => "None",
            ToneMap::Reinhard => "Reinhard",
            ToneMap::Aces => "ACES",
            ToneMap::Filmic => "Filmic",
        }
    }

    /// Integer selector passed to the `u_tone_map` shader uniform. Must match
    /// the branch numbering in `apply_tone_map()` (0=None, 1=Reinhard, 2=ACES,
    /// 3=Filmic).
    pub fn to_int(&self) -> i32 {
        match self {
            ToneMap::None => 0,
            ToneMap::Reinhard => 1,
            ToneMap::Aces => 2,
            ToneMap::Filmic => 3,
        }
    }
}

/// Light and camera parameters for one 3D viewer leaf.
///
/// Angles are stored in **radians** (the UI presents them in degrees and
/// converts). `Default` reproduces the viewer's original hard-coded behavior:
/// a single white directional light from `Vec3::new(0.8, 1.0, 0.6).normalize()`
/// at intensity 3.0, and a 45° vertical field of view.
#[derive(Clone, Debug, PartialEq)]
pub struct Viewer3dSettings {
    /// Light azimuth (rotation around +Y), in radians.
    pub light_azimuth: f32,
    /// Light elevation above the horizon, in radians (π/2 = straight up).
    pub light_elevation: f32,
    /// Light color as linear RGB (each channel 0..1), before intensity scaling.
    pub light_color: [f32; 3],
    /// Scalar multiplier applied to `light_color` to form the light radiance.
    pub light_intensity: f32,
    /// Vertical field of view, in **degrees** (converted to radians for the camera).
    pub fov_y_degrees: f32,
    /// Height-displacement amount applied along the surface normal when a Height
    /// texture is bound. `0.0` disables displacement; meshes span ~2 world units.
    pub height_scale: f32,
    /// Multiplier on the environment (IBL) contribution: `0.0` disables ambient
    /// lighting, `1.0` is the procedural sky's authored radiance.
    pub env_intensity: f32,
    /// Draw the procedural sky (with sun disc) behind the mesh. Off by default,
    /// preserving the theme `grid_bg` background painted by egui.
    pub show_skybox: bool,
    /// UV repeat multiplier applied to the material textures (`a_uv * u_uv_tiling`
    /// in the vertex shader). `1.0` shows one copy across the mesh; higher values
    /// tile the texture. Values >1 on the sphere/cylinder wrap through the UV seam.
    pub uv_tiling: f32,
    /// Tone-mapping operator applied to the HDR render before gamma.
    pub tone_map: ToneMap,
    /// Draw the mesh as a wireframe overlay on top of the shaded fill.
    pub wireframe: bool,
    /// Cast directional shadows from the sun light. When on, an off-screen depth
    /// pass from the light's viewpoint feeds a percentage-closer-filtered shadow
    /// test that darkens only the *direct* Cook-Torrance term (IBL ambient and
    /// emissive stay lit). Default `true` — shadows read as the expected look for
    /// a lit material preview; turn off to skip the extra depth pass.
    pub shadows: bool,
}

impl Default for Viewer3dSettings {
    fn default() -> Self {
        Self {
            // These azimuth/elevation values reproduce the old hard-coded
            // direction `Vec3::new(0.8, 1.0, 0.6).normalize()` when fed through
            // `light_direction` (verified by unit test). Elevation = asin(y) and
            // azimuth = atan2(x, z) of that normalized vector:
            //   normalized ≈ (0.56569, 0.70711, 0.42426)
            //   elevation  = asin(0.70711) ≈ 0.7853982 rad (45°)
            //   azimuth    = atan2(0.8, 0.6) ≈ 0.9272952 rad (~53.13°)
            light_azimuth: 0.9272952,
            light_elevation: 0.7853982,
            // White light, matching the previous shader's implicit white radiance.
            light_color: [1.0, 1.0, 1.0],
            // Old shader baked a radiance of 3.0 into the direct lighting term.
            light_intensity: 3.0,
            // Old camera used a 45° vertical FOV.
            fov_y_degrees: 45.0,
            // A gentle default relief; meshes span ~2 world units.
            height_scale: 0.15,
            // Full-strength environment lighting by default.
            env_intensity: 1.0,
            // Skybox off by default: keeps the current theme-background look.
            show_skybox: false,
            // One texture copy across the mesh by default.
            uv_tiling: 1.0,
            // ACES tone mapping by default (see ToneMap::default).
            tone_map: ToneMap::default(),
            // Wireframe overlay off by default.
            wireframe: false,
            // Directional shadows on by default (the expected lit-preview look).
            shadows: true,
        }
    }
}

/// Convert spherical light angles into a unit direction vector.
///
/// The returned vector points *toward* the light (the convention the shader's
/// `u_light_dir` expects). Uses a Y-up spherical parameterization:
///
/// - `elevation` is the angle above the horizontal plane; `π/2` yields `+Y`.
/// - `azimuth` rotates the horizontal component around the +Y axis, measured
///   from +Z toward +X (so `x = cos(el) * sin(az)`, `z = cos(el) * cos(az)`).
///
/// The result is normalized, so it is always unit length even at the pole where
/// `cos(elevation)` underflows to a tiny non-zero value in `f32`.
pub fn light_direction(azimuth: f32, elevation: f32) -> Vec3 {
    let cos_el = elevation.cos();
    Vec3::new(
        cos_el * azimuth.sin(),
        elevation.sin(),
        cos_el * azimuth.cos(),
    )
    .normalize()
}

#[cfg(test)]
#[path = "viewer_settings_tests.rs"]
mod tests;
