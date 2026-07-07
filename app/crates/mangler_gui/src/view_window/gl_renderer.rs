use std::collections::HashMap;

use glow::HasContext;
use mangler_core::float_image::FloatImage;

use super::arcball_camera::ArcballCamera;
use super::viewer_settings::ToneMap;
use super::environment::{
    build_irradiance_lut, build_specular_lut, GROUND, HORIZON, IRRADIANCE_SIZE, SPECULAR_SIZE,
    ZENITH,
};

const VERTEX_SHADER: &str = include_str!("vertex.glsl");
const FRAGMENT_SHADER: &str = include_str!("fragment.glsl");

// --- Shared sky / tone-map GLSL ---
//
// The skybox background must show *exactly* the environment that lit the
// material, so the GLSL sky gradient is generated from the same Rust constants
// (`environment::{ZENITH, HORIZON, GROUND}`) the CPU LUT prefilter integrates —
// built via `format!()`, the two definitions cannot drift apart.
//
// The mesh fragment shader itself does not need `sky_radiance` (its ambient
// term now comes from the prefiltered LUTs), so for now the shared string is
// only spliced into the sky shader. It is still structured as a shared
// constant because Phase 4's tone-map selector must be included into BOTH
// fragment sources the same way.

/// GLSL definition of `vec3 sky_radiance(vec3 dir)`, generated from the
/// environment constants. Matches `environment::sky_radiance` exactly: above
/// the horizon lerp HORIZON→ZENITH by smoothstep(dir.y), below lerp
/// HORIZON→GROUND — and, like the CPU version, contains **no sun term** (the
/// sun disc is added separately in the sky fragment shader).
fn sky_glsl() -> String {
    format!(
        r#"
const vec3 SKY_ZENITH = vec3({zx:?}, {zy:?}, {zz:?});
const vec3 SKY_HORIZON = vec3({hx:?}, {hy:?}, {hz:?});
const vec3 SKY_GROUND = vec3({gx:?}, {gy:?}, {gz:?});

// Procedural sky gradient; must match environment.rs's sky_radiance().
vec3 sky_radiance(vec3 dir) {{
    float y = normalize(dir).y;
    if (y >= 0.0) {{
        return mix(SKY_HORIZON, SKY_ZENITH, smoothstep(0.0, 1.0, y));
    }} else {{
        return mix(SKY_HORIZON, SKY_GROUND, smoothstep(0.0, 1.0, -y));
    }}
}}
"#,
        zx = ZENITH.x, zy = ZENITH.y, zz = ZENITH.z,
        hx = HORIZON.x, hy = HORIZON.y, hz = HORIZON.z,
        gx = GROUND.x, gy = GROUND.y, gz = GROUND.z,
    )
}

/// Tone-map GLSL shared by BOTH fragment shaders (mesh + sky) so the material
/// and the background behind it map HDR radiance to display identically.
///
/// This one string is spliced into the sky source via `format!()` and into the
/// mesh `fragment.glsl` via a placeholder-token replace (see
/// [`mesh_fragment_source`]). `apply_tone_map` implements the four selectable
/// operators; `mode` matches `ToneMap::to_int` (0=None, 1=Reinhard, 2=ACES,
/// 3=Filmic). The final `pow(1/2.2)` gamma step is deliberately kept SEPARATE —
/// each caller applies it after `apply_tone_map` so gamma runs for every mode.
const TONE_MAP_GLSL: &str = r#"
// Hable / Uncharted 2 filmic partial curve (shared by the Filmic branch below).
vec3 hable_partial(vec3 x) {
    const float A = 0.15; // shoulder strength
    const float B = 0.50; // linear strength
    const float C = 0.10; // linear angle
    const float D = 0.20; // toe strength
    const float E = 0.02; // toe numerator
    const float F = 0.30; // toe denominator
    return ((x * (A * x + C * B) + D * E) / (x * (A * x + B) + D * F)) - E / F;
}

// Tone-map HDR linear radiance to a [0,1] display range. `mode` selects the
// operator (kept in lockstep with ToneMap::to_int). Gamma is NOT applied here.
vec3 apply_tone_map(vec3 color, int mode) {
    if (mode == 1) {
        // Reinhard: the viewer's original operator.
        return color / (color + vec3(1.0));
    } else if (mode == 2) {
        // ACES filmic approximation (Narkowicz 2015 fit).
        const float a = 2.51;
        const float b = 0.03;
        const float c = 2.43;
        const float d = 0.59;
        const float e = 0.14;
        return clamp((color * (a * color + b)) / (color * (c * color + d) + e), 0.0, 1.0);
    } else if (mode == 3) {
        // Hable/Uncharted 2 filmic with a linear white point of 11.2.
        const float WHITE = 11.2;
        vec3 mapped = hable_partial(color);
        vec3 white_scale = vec3(1.0) / hable_partial(vec3(WHITE));
        return clamp(mapped * white_scale, 0.0, 1.0);
    } else {
        // None: no compression, just clamp so gamma below stays in range.
        return clamp(color, 0.0, 1.0);
    }
}
"#;

/// Vertex shader for the skybox background: a fullscreen triangle synthesized
/// from `gl_VertexID` (no vertex attributes — an empty VAO is bound, which GL
/// 3.3 core still requires). The output depth is forced to the far plane
/// (`z = w` → NDC z = 1.0) so the sky only fills pixels the mesh left at the
/// cleared depth (drawn with `LEQUAL`, depth writes off).
const SKY_VERTEX_SHADER: &str = r#"#version 330 core
out vec2 v_ndc;

void main() {
    // gl_VertexID 0,1,2 → (-1,-1), (3,-1), (-1,3): one CCW triangle covering
    // the whole viewport (clipped to it), the standard attribute-less trick.
    vec2 pos = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)) * 2.0 - 1.0;
    v_ndc = pos;
    gl_Position = vec4(pos, 1.0, 1.0); // z = w → depth exactly 1.0 (far plane)
}
"#;

/// Build the skybox fragment shader source by splicing the shared sky gradient
/// and tone-map snippets around the per-pixel view-ray reconstruction.
fn sky_fragment_source() -> String {
    format!(
        r#"#version 330 core
in vec2 v_ndc;

// Inverse of (projection * rotation-only view): unprojects an NDC point on the
// far plane straight to a world-space view direction (no translation, so the
// camera sits at the origin for this purpose — a skybox is at infinity).
uniform mat4 u_inv_view_proj;
uniform vec3 u_light_dir;   // unit direction toward the light (sun position)
uniform vec3 u_light_color; // light radiance (color * intensity)
uniform int u_tone_map;     // tone-map selector (matches ToneMap::to_int)

out vec4 frag_color;
{sky}
{tone_map}
void main() {{
    // Reconstruct the world-space ray through this pixel.
    vec4 world = u_inv_view_proj * vec4(v_ndc, 1.0, 1.0);
    vec3 dir = normalize(world.xyz / world.w);

    vec3 color = sky_radiance(dir);

    // Sun disc: a smoothstep-edged disc around the light direction, colored by
    // the light radiance. The 0.4 scale keeps the disc readable rather than a
    // white blowout at high intensities (the tone map compresses the rest).
    float cos_angle = dot(dir, normalize(u_light_dir));
    float sun = smoothstep(0.9995, 0.9999, cos_angle);
    color += u_light_color * (0.4 * sun);

    // Same tone map + gamma as the mesh shader, so sky and material match.
    color = apply_tone_map(color, u_tone_map);
    frag_color = vec4(pow(color, vec3(1.0 / 2.2)), 1.0);
}}
"#,
        sky = sky_glsl(),
        tone_map = TONE_MAP_GLSL,
    )
}

/// Placeholder token in `fragment.glsl` replaced by the shared [`TONE_MAP_GLSL`]
/// at load time. The mesh fragment shader is a full `.glsl` file (loaded via
/// `include_str!`) that is dense with `{`/`}` GLSL braces, so the sky shader's
/// `format!()` assembly would need every brace escaped. Injecting the shared
/// tone-map string via a comment-token replace is the cleaner choice here and
/// keeps `apply_tone_map` identical across both programs.
const TONE_MAP_PLACEHOLDER: &str = "//__TONE_MAP_GLSL__";

/// Build the mesh fragment shader source: the base `fragment.glsl` with the
/// shared tone-map GLSL spliced in at the placeholder token.
fn mesh_fragment_source() -> String {
    FRAGMENT_SHADER.replace(TONE_MAP_PLACEHOLDER, TONE_MAP_GLSL)
}

/// Number of floats per vertex: pos(3) + normal(3) + uv(2) + tangent(4) = 12
pub const VERTEX_STRIDE: usize = 12;

/// Which preview mesh to draw in the 3D viewer.
// `Hash`/`Eq` so `(MeshKind, MeshResolution)` can key the lazy mesh cache.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MeshKind {
    Plane,
    Sphere,
    Cube,
    RoundedCube,
    Cylinder,
    Torus,
}

impl MeshKind {
    pub const ALL: [MeshKind; 6] = [
        MeshKind::Plane,
        MeshKind::Sphere,
        MeshKind::Cube,
        MeshKind::RoundedCube,
        MeshKind::Cylinder,
        MeshKind::Torus,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            MeshKind::Plane => "Plane",
            MeshKind::Sphere => "Sphere",
            MeshKind::Cube => "Cube",
            MeshKind::RoundedCube => "Rounded Cube",
            MeshKind::Cylinder => "Cylinder",
            MeshKind::Torus => "Torus",
        }
    }
}

/// Tessellation level for the preview meshes. Higher levels give smoother
/// silhouettes and — more importantly for Phase 2 — enough vertices for the
/// vertex-shader height displacement to actually deform the surface (a coarse
/// mesh has too few vertices to sample the height field meaningfully).
///
/// The concrete subdivision numbers are chosen per mesh so all three kinds land
/// at a comparable triangle budget. `Default` is `Medium`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MeshResolution {
    Low,
    Medium,
    High,
}

impl Default for MeshResolution {
    fn default() -> Self {
        MeshResolution::Medium
    }
}

impl MeshResolution {
    /// All resolutions, ordered coarse → fine, for the UI combo box.
    pub const ALL: [MeshResolution; 3] =
        [MeshResolution::Low, MeshResolution::Medium, MeshResolution::High];

    /// Human-readable name for the resolution combo box.
    pub fn label(&self) -> &'static str {
        match self {
            MeshResolution::Low => "Low",
            MeshResolution::Medium => "Medium",
            MeshResolution::High => "High",
        }
    }

    /// Grid subdivisions per edge for the plane (`(subdiv+1)²` vertices).
    pub fn plane_subdiv(&self) -> u32 {
        match self {
            MeshResolution::Low => 64,
            MeshResolution::Medium => 128,
            MeshResolution::High => 256,
        }
    }

    /// `(slices, stacks)` for the UV sphere.
    pub fn sphere_slices_stacks(&self) -> (u32, u32) {
        match self {
            MeshResolution::Low => (64, 48),
            MeshResolution::Medium => (128, 96),
            MeshResolution::High => (256, 192),
        }
    }

    /// Per-face grid subdivisions for the cube (`(subdiv+1)²` vertices per face,
    /// six faces kept separate to preserve UV/tangent seams). Also drives the
    /// rounded cube (same subdivided-cube generation, then rounded via SDF).
    pub fn cube_subdiv(&self) -> u32 {
        match self {
            MeshResolution::Low => 16,
            MeshResolution::Medium => 32,
            MeshResolution::High => 64,
        }
    }

    /// `(segments, rings)` for the cylinder: `segments` around the circumference,
    /// `rings` up the side. Comparable triangle budget to the other meshes.
    pub fn cylinder_segments_rings(&self) -> (u32, u32) {
        match self {
            MeshResolution::Low => (64, 32),
            MeshResolution::Medium => (128, 64),
            MeshResolution::High => (256, 128),
        }
    }

    /// `(major_seg, minor_seg)` for the torus: `major_seg` around the main ring,
    /// `minor_seg` around the tube cross-section.
    pub fn torus_major_minor(&self) -> (u32, u32) {
        match self {
            MeshResolution::Low => (64, 32),
            MeshResolution::Medium => (128, 64),
            MeshResolution::High => (256, 128),
        }
    }
}

/// Index of each PBR texture channel (used as array index and GL texture unit).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextureChannel {
    Albedo = 0,
    Normal = 1,
    Roughness = 2,
    Metallic = 3,
    Height = 4,
    AmbientOcclusion = 5,
    Emissive = 6,
}

const CHANNEL_COUNT: usize = 7;

/// Texture unit for the diffuse irradiance LUT. Derived from `CHANNEL_COUNT` so
/// it always sits just above the material channels (units `0..CHANNEL_COUNT`);
/// adding the Emissive channel shifted these up automatically. After Phase 4
/// the budget is 7 material + 2 LUT = 9 of GL 3.3's guaranteed 16.
const IRRADIANCE_TEX_UNIT: u32 = CHANNEL_COUNT as u32;
/// Texture unit for the prefiltered specular LUT.
const SPECULAR_TEX_UNIT: u32 = CHANNEL_COUNT as u32 + 1;

/// Per-frame render inputs, snapshotted on the UI thread and moved into the
/// egui_glow paint callback (which runs where the GL context is available).
///
/// Kept `Clone` so the UI can build it and hand an owned copy to the callback.
/// This struct is the growth point for later phases (env intensity, tone map,
/// height scale, wireframe, etc.) — add fields here rather than widening the
/// `render()` argument list.
#[derive(Clone)]
pub struct RenderParams {
    /// Camera to view the scene from (includes pan `target`, orbit, zoom, FOV).
    pub camera: ArcballCamera,
    /// Which preview mesh to draw.
    pub mesh_kind: MeshKind,
    /// Tessellation level of the preview mesh (selects the cached mesh variant).
    pub mesh_resolution: MeshResolution,
    /// Unit direction pointing toward the light.
    pub light_dir: glam::Vec3,
    /// Light radiance: `light_color * light_intensity` (premultiplied).
    pub light_color: glam::Vec3,
    /// Vertex-displacement amount applied along the surface normal when a Height
    /// texture is bound. `0.0` disables displacement; meshes span ~2 world units,
    /// so values around `0.15` give a Substance-like relief.
    pub height_scale: f32,
    /// Multiplier on the environment (IBL) contribution: `0.0` disables ambient
    /// lighting entirely, `1.0` is the sky's authored radiance.
    pub env_intensity: f32,
    /// Draw the procedural sky (with sun disc) behind the mesh. When `false`,
    /// the background stays whatever egui painted (the theme's `grid_bg`).
    pub show_skybox: bool,
    /// UV repeat multiplier (`a_uv * u_uv_tiling` in the vertex shader).
    pub uv_tiling: f32,
    /// Tone-mapping operator selector (fed to `u_tone_map` in both programs).
    pub tone_map: ToneMap,
    /// Overlay the mesh as a wireframe on top of the shaded fill.
    pub wireframe: bool,
    /// Wireframe line color as gamma-space RGBA (from the active theme — never
    /// hardcoded). Ignored when `wireframe` is false.
    pub wire_color: [f32; 4],
}

struct TextureSlot {
    texture: Option<glow::Texture>,
    change_id: Option<String>,
    u_sampler: Option<glow::UniformLocation>,
    u_has: Option<glow::UniformLocation>,
}

struct Mesh {
    vao: glow::VertexArray,
    _vbo: glow::Buffer,
    _ebo: glow::Buffer,
    index_count: i32,
}

pub struct GlRenderer {
    program: glow::Program,
    /// Lazy cache of uploaded meshes keyed by `(kind, resolution)`. Built on
    /// first use inside `render()` (mesh generation + GL buffer upload both need
    /// the GL context). The cache is bounded — 3 kinds × 3 resolutions = 9
    /// entries at most — so no buffer-delete/eviction plumbing is needed.
    meshes: HashMap<(MeshKind, MeshResolution), Mesh>,
    // Uniform locations (Option because the GLSL compiler may optimize out unused uniforms)
    u_model: Option<glow::UniformLocation>,
    u_view: Option<glow::UniformLocation>,
    u_projection: Option<glow::UniformLocation>,
    u_light_dir: Option<glow::UniformLocation>,
    u_light_color: Option<glow::UniformLocation>,
    u_camera_pos: Option<glow::UniformLocation>,
    /// Vertex-shader displacement scale (`u_height_scale`).
    u_height_scale: Option<glow::UniformLocation>,
    /// UV tiling multiplier (`u_uv_tiling`, vertex shader).
    u_uv_tiling: Option<glow::UniformLocation>,
    /// Tone-map selector (`u_tone_map`, mesh program).
    u_tone_map: Option<glow::UniformLocation>,
    /// Wireframe flat-color RGBA (`u_flat_color`) and its enable flag
    /// (`u_use_flat_color`) — set only for the wireframe overlay pass.
    u_flat_color: Option<glow::UniformLocation>,
    u_use_flat_color: Option<glow::UniformLocation>,
    // IBL uniforms (mesh program)
    u_irradiance_lut: Option<glow::UniformLocation>,
    u_specular_lut: Option<glow::UniformLocation>,
    u_env_intensity: Option<glow::UniformLocation>,
    /// CPU-prefiltered diffuse irradiance LUT (64×1 RGBA32F), built once at init.
    irradiance_tex: glow::Texture,
    /// CPU-prefiltered specular LUT (64×64 RGBA32F over (R.y, roughness)).
    specular_tex: glow::Texture,
    // Skybox background pass (fullscreen triangle, drawn after the mesh)
    sky_program: glow::Program,
    /// Empty VAO for the attribute-less fullscreen triangle (GL 3.3 core
    /// requires *some* VAO bound even when no attributes are read).
    sky_vao: glow::VertexArray,
    u_sky_inv_view_proj: Option<glow::UniformLocation>,
    u_sky_light_dir: Option<glow::UniformLocation>,
    u_sky_light_color: Option<glow::UniformLocation>,
    u_sky_tone_map: Option<glow::UniformLocation>,
    // PBR texture slots
    slots: Vec<TextureSlot>,
    // Max anisotropic filter level. 1.0 means anisotropic is unavailable and
    // we fall back to trilinear.
    max_anisotropy: f32,
}

impl GlRenderer {
    pub fn new(gl: &glow::Context) -> Self {
        unsafe {
            // Mesh program: fragment source is assembled from fragment.glsl with
            // the shared tone-map GLSL spliced in at the placeholder token.
            let program = create_program(gl, VERTEX_SHADER, &mesh_fragment_source());

            let u_model = gl.get_uniform_location(program, "u_model");
            let u_view = gl.get_uniform_location(program, "u_view");
            let u_projection = gl.get_uniform_location(program, "u_projection");
            let u_light_dir = gl.get_uniform_location(program, "u_light_dir");
            let u_light_color = gl.get_uniform_location(program, "u_light_color");
            let u_camera_pos = gl.get_uniform_location(program, "u_camera_pos");
            let u_height_scale = gl.get_uniform_location(program, "u_height_scale");
            let u_uv_tiling = gl.get_uniform_location(program, "u_uv_tiling");
            let u_tone_map = gl.get_uniform_location(program, "u_tone_map");
            let u_flat_color = gl.get_uniform_location(program, "u_flat_color");
            let u_use_flat_color = gl.get_uniform_location(program, "u_use_flat_color");
            let u_irradiance_lut = gl.get_uniform_location(program, "u_irradiance_lut");
            let u_specular_lut = gl.get_uniform_location(program, "u_specular_lut");
            let u_env_intensity = gl.get_uniform_location(program, "u_env_intensity");

            // One entry per TextureChannel (order MUST match the enum discriminants).
            let sampler_names = [
                "u_albedo_tex", "u_normal_tex", "u_roughness_tex",
                "u_metallic_tex", "u_height_tex", "u_ao_tex", "u_emissive_tex",
            ];
            let has_names = [
                "u_has_albedo", "u_has_normal", "u_has_roughness",
                "u_has_metallic", "u_has_height", "u_has_ao", "u_has_emissive",
            ];

            let mut slots = Vec::with_capacity(CHANNEL_COUNT);
            for i in 0..CHANNEL_COUNT {
                slots.push(TextureSlot {
                    texture: None,
                    change_id: None,
                    u_sampler: gl.get_uniform_location(program, sampler_names[i]),
                    u_has: gl.get_uniform_location(program, has_names[i]),
                });
            }

            // Meshes are generated + uploaded lazily on first use in `render()`
            // (both steps need the GL context, which is available there too).
            let meshes = HashMap::new();

            // Prefilter the procedural environment on the CPU (one-time cost,
            // well under 100 ms — see the timing smoke test in
            // environment_tests.rs) and upload the results as small float LUTs.
            let irradiance_data = build_irradiance_lut();
            let irradiance_tex =
                upload_lut_texture(gl, IRRADIANCE_SIZE as i32, 1, &irradiance_data);
            let specular_data = build_specular_lut();
            let specular_tex = upload_lut_texture(
                gl,
                SPECULAR_SIZE as i32,
                SPECULAR_SIZE as i32,
                &specular_data,
            );

            // Skybox background program: attribute-less fullscreen triangle.
            let sky_program = create_program(gl, SKY_VERTEX_SHADER, &sky_fragment_source());
            let sky_vao = gl.create_vertex_array().unwrap();
            let u_sky_inv_view_proj = gl.get_uniform_location(sky_program, "u_inv_view_proj");
            let u_sky_light_dir = gl.get_uniform_location(sky_program, "u_light_dir");
            let u_sky_light_color = gl.get_uniform_location(sky_program, "u_light_color");
            let u_sky_tone_map = gl.get_uniform_location(sky_program, "u_tone_map");

            // Query max anisotropy. Core since GL 4.6, ubiquitously available
            // via GL_EXT_texture_filter_anisotropic before that (same enum value).
            // If unsupported the query leaves the value at 0; treat anything
            // below 2.0 as "not available" and stay on trilinear.
            let queried = gl.get_parameter_f32(glow::MAX_TEXTURE_MAX_ANISOTROPY);
            let max_anisotropy = if queried >= 2.0 { queried.min(16.0) } else { 1.0 };

            Self {
                program,
                meshes,
                u_model,
                u_view,
                u_projection,
                u_light_dir,
                u_light_color,
                u_camera_pos,
                u_height_scale,
                u_uv_tiling,
                u_tone_map,
                u_flat_color,
                u_use_flat_color,
                u_irradiance_lut,
                u_specular_lut,
                u_env_intensity,
                irradiance_tex,
                specular_tex,
                sky_program,
                sky_vao,
                u_sky_inv_view_proj,
                u_sky_light_dir,
                u_sky_light_color,
                u_sky_tone_map,
                slots,
                max_anisotropy,
            }
        }
    }

    /// Upload a FloatImage to a specific PBR channel. Skips if change_id hasn't changed.
    pub fn upload_texture(
        &mut self,
        gl: &glow::Context,
        channel: TextureChannel,
        image: &FloatImage,
        change_id: &str,
    ) {
        let slot = &mut self.slots[channel as usize];
        if slot.change_id.as_deref() == Some(change_id) {
            return;
        }

        unsafe {
            if let Some(tex) = slot.texture.take() {
                gl.delete_texture(tex);
            }

            let tex = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));

            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            if self.max_anisotropy > 1.0 {
                gl.tex_parameter_f32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAX_ANISOTROPY,
                    self.max_anisotropy,
                );
            }

            let width = image.width() as i32;
            let height = image.height() as i32;
            let rgba_data = to_rgba_f32(image);

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA32F as i32,
                width,
                height,
                0,
                glow::RGBA,
                glow::FLOAT,
                glow::PixelUnpackData::Slice(Some(cast_slice_to_bytes(&rgba_data))),
            );

            gl.generate_mipmap(glow::TEXTURE_2D);
            gl.bind_texture(glow::TEXTURE_2D, None);

            slot.texture = Some(tex);
            slot.change_id = Some(change_id.to_string());
        }
    }

    /// Returns true if a channel's texture needs re-uploading.
    pub fn needs_update(&self, channel: TextureChannel, change_id: &str) -> bool {
        self.slots[channel as usize].change_id.as_deref() != Some(change_id)
    }

    /// Delete the GL texture bound to `channel` (if any) and reset the slot back
    /// to its initial "no texture" state, so `render()`'s u_has_* flags (which
    /// key off `slot.texture`) report false again. Idempotent: calling this twice
    /// in a row, or on a slot that was never uploaded, is a no-op the second time.
    pub fn clear_texture(&mut self, gl: &glow::Context, channel: TextureChannel) {
        let slot = &mut self.slots[channel as usize];
        unsafe {
            if let Some(tex) = slot.texture.take() {
                gl.delete_texture(tex);
            }
        }
        slot.change_id = None;
    }

    /// True if `channel` currently has an uploaded GL texture (and therefore
    /// would render with u_has_* = 1 this frame). `texture` and `change_id` are
    /// always set/cleared together, so checking `change_id` is equivalent to
    /// checking `texture` and matches `needs_update`'s convention.
    pub fn has_texture(&self, channel: TextureChannel) -> bool {
        self.slots[channel as usize].change_id.is_some()
    }

    /// Render the scene. `viewport` is [x, y, width, height] in physical pixels.
    /// All per-frame inputs (camera, mesh, light) come from `params`.
    ///
    /// Takes `&mut self` because the mesh cache may be populated on first use of
    /// a given `(kind, resolution)` pair.
    pub fn render(&mut self, gl: &glow::Context, viewport: [i32; 4], params: &RenderParams) {
        let camera = &params.camera;
        let mesh_kind = params.mesh_kind;
        let [vp_x, vp_y, vp_w, vp_h] = viewport;
        if vp_w <= 0 || vp_h <= 0 {
            return;
        }

        unsafe {
            // Set 3D rendering state
            gl.viewport(vp_x, vp_y, vp_w, vp_h);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            // Explicit front-face winding: egui_glow may leave this set to CW
            // between frames, which silently inverts cull results.
            gl.front_face(glow::CCW);
            // MSAA on the default framebuffer (sample count set in NativeOptions).
            gl.enable(glow::MULTISAMPLE);
            gl.disable(glow::BLEND);

            // Clear depth in our region
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(vp_x, vp_y, vp_w, vp_h);
            gl.clear(glow::DEPTH_BUFFER_BIT);

            gl.use_program(Some(self.program));

            // Matrices
            let model = glam::Mat4::IDENTITY;
            let aspect = vp_w as f32 / vp_h as f32;
            let view = camera.view_matrix();
            let projection = camera.projection_matrix(aspect);

            gl.uniform_matrix_4_f32_slice(self.u_model.as_ref(), false, &model.to_cols_array());
            gl.uniform_matrix_4_f32_slice(self.u_view.as_ref(), false, &view.to_cols_array());
            gl.uniform_matrix_4_f32_slice(self.u_projection.as_ref(), false, &projection.to_cols_array());

            // Directional light: direction + premultiplied color/intensity radiance,
            // both supplied by the caller via `RenderParams`.
            let light_dir = params.light_dir;
            gl.uniform_3_f32(self.u_light_dir.as_ref(), light_dir.x, light_dir.y, light_dir.z);
            let light_color = params.light_color;
            gl.uniform_3_f32(
                self.u_light_color.as_ref(),
                light_color.x,
                light_color.y,
                light_color.z,
            );

            let eye = camera.eye_position();
            gl.uniform_3_f32(self.u_camera_pos.as_ref(), eye.x, eye.y, eye.z);

            // Vertex-displacement scale (sampled per-vertex in the vertex shader
            // when a Height texture is bound).
            gl.uniform_1_f32(self.u_height_scale.as_ref(), params.height_scale);
            // UV tiling multiplier and tone-map selector.
            gl.uniform_1_f32(self.u_uv_tiling.as_ref(), params.uv_tiling);
            gl.uniform_1_i32(self.u_tone_map.as_ref(), params.tone_map.to_int());
            // Default to the shaded (non-flat) path; the wireframe pass below
            // flips u_use_flat_color on for its overlay draw and back off after.
            gl.uniform_1_i32(self.u_use_flat_color.as_ref(), 0);

            // Bind all PBR texture slots
            for (i, slot) in self.slots.iter().enumerate() {
                let unit = glow::TEXTURE0 + i as u32;
                gl.active_texture(unit);
                let has = if let Some(tex) = slot.texture {
                    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                    1
                } else {
                    gl.bind_texture(glow::TEXTURE_2D, None);
                    0
                };
                gl.uniform_1_i32(slot.u_sampler.as_ref(), i as i32);
                gl.uniform_1_i32(slot.u_has.as_ref(), has);
            }

            // Bind the environment LUTs alongside the material slots every
            // frame (egui's own texture binds clobber the units between frames).
            gl.active_texture(glow::TEXTURE0 + IRRADIANCE_TEX_UNIT);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.irradiance_tex));
            gl.active_texture(glow::TEXTURE0 + SPECULAR_TEX_UNIT);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.specular_tex));
            gl.uniform_1_i32(self.u_irradiance_lut.as_ref(), IRRADIANCE_TEX_UNIT as i32);
            gl.uniform_1_i32(self.u_specular_lut.as_ref(), SPECULAR_TEX_UNIT as i32);
            gl.uniform_1_f32(self.u_env_intensity.as_ref(), params.env_intensity);

            // Fetch (or lazily build) the mesh for this kind+resolution. Mesh
            // generation and buffer upload both happen here, where the GL
            // context is live.
            let key = (mesh_kind, params.mesh_resolution);
            if !self.meshes.contains_key(&key) {
                let data = match mesh_kind {
                    MeshKind::Plane => generate_plane(params.mesh_resolution.plane_subdiv()),
                    MeshKind::Sphere => {
                        let (slices, stacks) = params.mesh_resolution.sphere_slices_stacks();
                        generate_sphere(slices, stacks)
                    }
                    MeshKind::Cube => generate_cube(params.mesh_resolution.cube_subdiv()),
                    MeshKind::RoundedCube => {
                        generate_rounded_cube(params.mesh_resolution.cube_subdiv())
                    }
                    MeshKind::Cylinder => {
                        let (segments, rings) = params.mesh_resolution.cylinder_segments_rings();
                        generate_cylinder(segments, rings)
                    }
                    MeshKind::Torus => {
                        let (major, minor) = params.mesh_resolution.torus_major_minor();
                        generate_torus(major, minor)
                    }
                };
                let mesh = upload_mesh(gl, &data);
                self.meshes.insert(key, mesh);
            }
            let mesh = self.meshes.get(&key).unwrap();
            gl.bind_vertex_array(Some(mesh.vao));
            gl.draw_elements(glow::TRIANGLES, mesh.index_count, glow::UNSIGNED_INT, 0);

            // Wireframe overlay: redraw the same mesh in line polygon mode with a
            // flat theme color. `polygon_offset(-1,-1)` pulls the lines slightly
            // toward the camera so they sit cleanly on top of the filled faces
            // instead of z-fighting them.
            //
            // NOTE: `polygon_mode` / `POLYGON_OFFSET_LINE` are native-GL only —
            // glow's WebGL backend panics on them. This app is desktop-only; a
            // web build would need a barycentric-coordinate wireframe trick.
            if params.wireframe {
                let [r, g, b, a] = params.wire_color;
                gl.uniform_4_f32(self.u_flat_color.as_ref(), r, g, b, a);
                gl.uniform_1_i32(self.u_use_flat_color.as_ref(), 1);
                gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
                gl.enable(glow::POLYGON_OFFSET_LINE);
                gl.polygon_offset(-1.0, -1.0);
                gl.draw_elements(glow::TRIANGLES, mesh.index_count, glow::UNSIGNED_INT, 0);
                // Restore fill rasterization + disable the offset before the
                // skybox and egui draw, and turn the flat-color path back off.
                gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
                gl.disable(glow::POLYGON_OFFSET_LINE);
                gl.uniform_1_i32(self.u_use_flat_color.as_ref(), 0);
            }

            // Skybox background: drawn AFTER the mesh so early-z rejects the
            // covered pixels. The fullscreen triangle sits exactly on the far
            // plane (VS emits z = w), which equals the cleared depth — hence
            // LEQUAL — and depth writes are disabled so the pass leaves the
            // depth buffer untouched.
            if params.show_skybox {
                gl.use_program(Some(self.sky_program));

                // Rotation-only view: drop the translation column so the sky is
                // anchored to the camera (a skybox is at infinity — panning or
                // orbiting must never parallax it).
                let view_rot = glam::Mat4::from_mat3(glam::Mat3::from_mat4(view));
                let inv_view_proj = (projection * view_rot).inverse();
                gl.uniform_matrix_4_f32_slice(
                    self.u_sky_inv_view_proj.as_ref(),
                    false,
                    &inv_view_proj.to_cols_array(),
                );
                gl.uniform_3_f32(
                    self.u_sky_light_dir.as_ref(),
                    light_dir.x,
                    light_dir.y,
                    light_dir.z,
                );
                gl.uniform_3_f32(
                    self.u_sky_light_color.as_ref(),
                    light_color.x,
                    light_color.y,
                    light_color.z,
                );
                // Sky tone-maps with the same operator as the mesh, so the
                // background and the material always match.
                gl.uniform_1_i32(self.u_sky_tone_map.as_ref(), params.tone_map.to_int());

                gl.depth_func(glow::LEQUAL);
                gl.depth_mask(false);
                gl.bind_vertex_array(Some(self.sky_vao)); // empty VAO (attribute-less draw)
                gl.draw_arrays(glow::TRIANGLES, 0, 3);
                // Restore the depth state the mesh pass (and next frame) expects.
                gl.depth_mask(true);
                gl.depth_func(glow::LESS);
            }

            // Restore state for egui's glow renderer
            gl.bind_vertex_array(None);
            gl.use_program(None);
            for i in 0..CHANNEL_COUNT {
                gl.active_texture(glow::TEXTURE0 + i as u32);
                gl.bind_texture(glow::TEXTURE_2D, None);
            }
            // Also unbind the LUT units so no stale float textures leak into
            // egui's own texture binds.
            gl.active_texture(glow::TEXTURE0 + IRRADIANCE_TEX_UNIT);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.active_texture(glow::TEXTURE0 + SPECULAR_TEX_UNIT);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.active_texture(glow::TEXTURE0);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
        }
    }

}

// --- Helpers ---

unsafe fn create_program(gl: &glow::Context, vert_src: &str, frag_src: &str) -> glow::Program {
    let program = gl.create_program().unwrap();

    let vert = gl.create_shader(glow::VERTEX_SHADER).unwrap();
    gl.shader_source(vert, vert_src);
    gl.compile_shader(vert);
    if !gl.get_shader_compile_status(vert) {
        panic!("Vertex shader error: {}", gl.get_shader_info_log(vert));
    }

    let frag = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
    gl.shader_source(frag, frag_src);
    gl.compile_shader(frag);
    if !gl.get_shader_compile_status(frag) {
        panic!("Fragment shader error: {}", gl.get_shader_info_log(frag));
    }

    gl.attach_shader(program, vert);
    gl.attach_shader(program, frag);
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!("Program link error: {}", gl.get_program_info_log(program));
    }

    gl.delete_shader(vert);
    gl.delete_shader(frag);

    program
}

/// Upload a CPU-built IBL LUT as an RGBA32F `TEXTURE_2D`: `CLAMP_TO_EDGE` on
/// both axes (LUT coordinates must not wrap), bilinear filtering, no mips
/// (the data is already the filtered result — mipping it would re-blur it).
/// `data` is interleaved RGBA, `width * height * 4` floats.
unsafe fn upload_lut_texture(
    gl: &glow::Context,
    width: i32,
    height: i32,
    data: &[f32],
) -> glow::Texture {
    debug_assert_eq!(data.len(), (width * height * 4) as usize);

    let tex = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA32F as i32,
        width,
        height,
        0,
        glow::RGBA,
        glow::FLOAT,
        glow::PixelUnpackData::Slice(Some(cast_slice_to_bytes(data))),
    );
    gl.bind_texture(glow::TEXTURE_2D, None);
    tex
}

/// Upload interleaved vertex/index data as a `Mesh` with all attrib pointers set.
/// Vertex layout matches [`VERTEX_STRIDE`]: pos(3) + normal(3) + uv(2) + tangent(4).
unsafe fn upload_mesh(gl: &glow::Context, data: &(Vec<f32>, Vec<u32>)) -> Mesh {
    let (vertices, indices) = data;

    let vao = gl.create_vertex_array().unwrap();
    gl.bind_vertex_array(Some(vao));

    let vbo = gl.create_buffer().unwrap();
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
    gl.buffer_data_u8_slice(
        glow::ARRAY_BUFFER,
        cast_slice_to_bytes(vertices),
        glow::STATIC_DRAW,
    );

    let ebo = gl.create_buffer().unwrap();
    gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
    gl.buffer_data_u8_slice(
        glow::ELEMENT_ARRAY_BUFFER,
        cast_slice_to_bytes(indices),
        glow::STATIC_DRAW,
    );

    let stride = VERTEX_STRIDE as i32 * std::mem::size_of::<f32>() as i32;
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
    gl.enable_vertex_attrib_array(1);
    gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 3 * 4);
    gl.enable_vertex_attrib_array(2);
    gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, stride, 6 * 4);
    gl.enable_vertex_attrib_array(3);
    gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, stride, 8 * 4);

    gl.bind_vertex_array(None);
    gl.bind_buffer(glow::ARRAY_BUFFER, None);

    Mesh {
        vao,
        _vbo: vbo,
        _ebo: ebo,
        index_count: indices.len() as i32,
    }
}

/// 2×2 plane centered at origin, facing +Z, tessellated into a
/// `(subdiv+1)² grid`. Single-sided; back-face cull hides it when the camera
/// orbits behind. The extra vertices exist so the vertex shader can displace
/// the surface along the height field (Phase 2).
///
/// UV convention (preserved from the original flat quad): FloatImage stores rows
/// top-to-bottom but OpenGL samples v=0 from the first uploaded row (texel y=0),
/// so a naïve (0,0) at the bottom-left would render the image upside-down. We
/// therefore flip V: `u = (x+1)/2`, `v = (1-y)/2`, which puts (0,0) at the
/// top-left corner (−1,+1) and (0,1) at the bottom-left (−1,−1) — identical to
/// the original 4-vertex quad. Tangent is +X, so the bitangent `cross(N,T)*w`
/// comes out +Y (matching the original), consistent across the whole grid.
fn generate_plane(subdiv: u32) -> (Vec<f32>, Vec<u32>) {
    // At least one cell so we always emit a valid quad.
    let subdiv = subdiv.max(1);
    let steps = subdiv + 1; // vertices per edge
    let mut vertices = Vec::with_capacity((steps * steps) as usize * VERTEX_STRIDE);
    let mut indices = Vec::with_capacity((subdiv * subdiv) as usize * 6);

    // Row `i` runs along +Y (y from −1 to +1); column `j` runs along +X.
    for i in 0..steps {
        for j in 0..steps {
            let fx = j as f32 / subdiv as f32; // 0..1 across +X
            let fy = i as f32 / subdiv as f32; // 0..1 across +Y
            let x = fx * 2.0 - 1.0;
            let y = fy * 2.0 - 1.0;
            let u = fx;
            let v = 1.0 - fy; // flip V (see doc comment)
            // position
            vertices.extend_from_slice(&[x, y, 0.0]);
            // normal (+Z)
            vertices.extend_from_slice(&[0.0, 0.0, 1.0]);
            // uv
            vertices.extend_from_slice(&[u, v]);
            // tangent (+X, w=1 → bitangent +Y)
            vertices.extend_from_slice(&[1.0, 0.0, 0.0, 1.0]);
        }
    }

    // Two CCW triangles per cell (as seen from +Z), matching the original
    // BL,BR,TR / BL,TR,TL winding.
    for i in 0..subdiv {
        for j in 0..subdiv {
            let bl = i * steps + j;
            let br = bl + 1;
            let tl = bl + steps;
            let tr = tl + 1;
            indices.extend_from_slice(&[bl, br, tr, bl, tr, tl]);
        }
    }

    (vertices, indices)
}

/// Cube with extents ±1 (matches sphere diameter). Each of the six faces is a
/// separate `(subdiv+1)²` grid so UVs/normals/tangents don't cross seams.
/// Windings are CCW from outside. Like the plane, the extra vertices exist so
/// the vertex shader can displace along the height field.
///
/// **Known limitation:** because faces are kept separate (non-watertight UVs),
/// displacement can crack open the cube edges when the height field differs
/// across a seam — the same behavior as Substance Designer on non-watertight
/// UVs. This is accepted, not a bug.
fn generate_cube(subdiv: u32) -> (Vec<f32>, Vec<u32>) {
    // For each face: normal, tangent (along +U), and four corners (BL, BR, TR, TL)
    // in CCW order as seen from outside the cube.
    let faces = cube_faces();

    // Same UV convention as the flat quad's corners (BL,BR,TR,TL →
    // (0,1),(1,1),(1,0),(0,0)): u runs BL→BR, v is flipped so V=0 is the top
    // edge (TL/TR). Parameterize each face by (s,t): s∈[0,1] along BL→BR,
    // t∈[0,1] along BL→TL; then `u = s`, `v = 1 - t`.
    let subdiv = subdiv.max(1);
    let steps = subdiv + 1;

    let mut vertices = Vec::with_capacity(faces.len() * (steps * steps) as usize * VERTEX_STRIDE);
    let mut indices = Vec::with_capacity(faces.len() * (subdiv * subdiv) as usize * 6);

    for (normal, tangent, corners) in &faces {
        let base = (vertices.len() / VERTEX_STRIDE) as u32;
        let [bl, br, tr, tl] = corners;

        // (subdiv+1)² grid of bilinearly-interpolated corner positions.
        for i in 0..steps {
            for j in 0..steps {
                let s = j as f32 / subdiv as f32;
                let t = i as f32 / subdiv as f32;
                // Bilinear blend of the four corners.
                let px = bilerp(bl[0], br[0], tr[0], tl[0], s, t);
                let py = bilerp(bl[1], br[1], tr[1], tl[1], s, t);
                let pz = bilerp(bl[2], br[2], tr[2], tl[2], s, t);
                vertices.extend_from_slice(&[px, py, pz]);
                vertices.extend_from_slice(normal);
                vertices.extend_from_slice(&[s, 1.0 - t]);
                vertices.extend_from_slice(&[tangent[0], tangent[1], tangent[2], 1.0]);
            }
        }

        // Two CCW triangles per cell, same orientation as the corner ordering
        // (BL,BR,TR / BL,TR,TL) so the original per-face winding is preserved.
        for i in 0..subdiv {
            for j in 0..subdiv {
                let a = base + i * steps + j; // BL of this cell
                let b = a + 1; // BR
                let d = a + steps; // TL
                let c = d + 1; // TR
                indices.extend_from_slice(&[a, b, c, a, c, d]);
            }
        }
    }

    (vertices, indices)
}

/// Bilinear interpolation of a scalar over a quad whose corners are given in
/// BL, BR, TR, TL order, evaluated at `(s, t)` with `s` running BL→BR and `t`
/// running BL→TL.
fn bilerp(bl: f32, br: f32, tr: f32, tl: f32, s: f32, t: f32) -> f32 {
    let bottom = bl + (br - bl) * s; // BL→BR
    let top = tl + (tr - tl) * s; // TL→TR
    bottom + (top - bottom) * t
}

/// Generate a UV sphere with tangent vectors.
/// Vertex layout: [px, py, pz, nx, ny, nz, u, v, tx, ty, tz, tw] per vertex.
fn generate_sphere(slices: u32, stacks: u32) -> (Vec<f32>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for stack in 0..=stacks {
        let phi = std::f32::consts::PI * stack as f32 / stacks as f32;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();

        for slice in 0..=slices {
            let theta = 2.0 * std::f32::consts::PI * slice as f32 / slices as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let x = sin_phi * cos_theta;
            let y = cos_phi;
            let z = sin_phi * sin_theta;

            let u = slice as f32 / slices as f32;
            let v = stack as f32 / stacks as f32;

            // Tangent along U direction (dP/dtheta, normalized)
            // dP/dtheta = (-sin_phi*sin_theta, 0, sin_phi*cos_theta)
            // normalized = (-sin_theta, 0, cos_theta) when sin_phi != 0
            let (tx, ty, tz) = if sin_phi.abs() > 1e-6 {
                (-sin_theta, 0.0, cos_theta)
            } else {
                // At poles, pick an arbitrary tangent
                (1.0, 0.0, 0.0)
            };

            // position
            vertices.push(x);
            vertices.push(y);
            vertices.push(z);
            // normal
            vertices.push(x);
            vertices.push(y);
            vertices.push(z);
            // uv
            vertices.push(u);
            vertices.push(v);
            // tangent (xyz + w for bitangent sign)
            vertices.push(tx);
            vertices.push(ty);
            vertices.push(tz);
            vertices.push(1.0); // bitangent sign
        }
    }

    for stack in 0..stacks {
        for slice in 0..slices {
            let first = stack * (slices + 1) + slice;
            let second = first + slices + 1;

            // CCW from outside the sphere (matches glFrontFace(CCW) + cull BACK).
            indices.push(first);
            indices.push(first + 1);
            indices.push(second);

            indices.push(second);
            indices.push(first + 1);
            indices.push(second + 1);
        }
    }

    (vertices, indices)
}

/// The six cube faces as `(normal, tangent, [BL, BR, TR, TL] corners)` with the
/// corners in CCW order as seen from outside. Shared by [`generate_cube`] and
/// [`generate_rounded_cube`] so the two cannot drift apart.
fn cube_faces() -> [([f32; 3], [f32; 3], [[f32; 3]; 4]); 6] {
    [
        // +X
        ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [
            [1.0, -1.0,  1.0], [1.0, -1.0, -1.0], [1.0,  1.0, -1.0], [1.0,  1.0,  1.0],
        ]),
        // -X
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [
            [-1.0, -1.0, -1.0], [-1.0, -1.0,  1.0], [-1.0,  1.0,  1.0], [-1.0,  1.0, -1.0],
        ]),
        // +Y
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [
            [-1.0, 1.0,  1.0], [ 1.0, 1.0,  1.0], [ 1.0, 1.0, -1.0], [-1.0, 1.0, -1.0],
        ]),
        // -Y
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [
            [-1.0, -1.0, -1.0], [ 1.0, -1.0, -1.0], [ 1.0, -1.0,  1.0], [-1.0, -1.0,  1.0],
        ]),
        // +Z
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [
            [-1.0, -1.0, 1.0], [ 1.0, -1.0, 1.0], [ 1.0,  1.0, 1.0], [-1.0,  1.0, 1.0],
        ]),
        // -Z
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [
            [ 1.0, -1.0, -1.0], [-1.0, -1.0, -1.0], [-1.0,  1.0, -1.0], [ 1.0,  1.0, -1.0],
        ]),
    ]
}

/// Rounded cube: the same per-face subdivided grid as [`generate_cube`], with
/// every position pushed onto a rounded-box surface via the SDF clamp trick.
///
/// For each cube-surface point `p ∈ [-1,1]³`, let `core = clamp(p, -(1-r), 1-r)`
/// be the nearest point of the inner (shrunk) box; the rounded position is then
/// `core + r * normalize(p - core)` and the smooth outward normal is
/// `normalize(p - core)` (r = 0.15). Because a cube-surface point always has at
/// least one coordinate at ±1 (outside the ±(1-r) clamp box), `p - core` is
/// never zero, so the normalize is always well-defined.
///
/// The original **face UVs and face tangents are kept unchanged** — the vertex
/// shader's Gram-Schmidt step (`T -= dot(T,N)*N`) re-orthogonalizes each stored
/// tangent against the new rounded normal, so no per-vertex tangent fix-up is
/// needed here. Faces are still kept separate (non-watertight UVs), same seam
/// caveat as the flat cube.
fn generate_rounded_cube(subdiv: u32) -> (Vec<f32>, Vec<u32>) {
    // Corner radius, in the mesh's ±1 object space (matches the plan).
    const R: f32 = 0.15;
    let limit = 1.0 - R;

    let faces = cube_faces();
    let subdiv = subdiv.max(1);
    let steps = subdiv + 1;

    let mut vertices = Vec::with_capacity(faces.len() * (steps * steps) as usize * VERTEX_STRIDE);
    let mut indices = Vec::with_capacity(faces.len() * (subdiv * subdiv) as usize * 6);

    for (_normal, tangent, corners) in &faces {
        let base = (vertices.len() / VERTEX_STRIDE) as u32;
        let [bl, br, tr, tl] = corners;

        for i in 0..steps {
            for j in 0..steps {
                let s = j as f32 / subdiv as f32;
                let t = i as f32 / subdiv as f32;
                // Flat cube-surface position (bilinear blend of the corners).
                let px = bilerp(bl[0], br[0], tr[0], tl[0], s, t);
                let py = bilerp(bl[1], br[1], tr[1], tl[1], s, t);
                let pz = bilerp(bl[2], br[2], tr[2], tl[2], s, t);

                // SDF rounded-box projection.
                let cx = px.clamp(-limit, limit);
                let cy = py.clamp(-limit, limit);
                let cz = pz.clamp(-limit, limit);
                let dx = px - cx;
                let dy = py - cy;
                let dz = pz - cz;
                let inv_len = 1.0 / (dx * dx + dy * dy + dz * dz).sqrt();
                let nx = dx * inv_len;
                let ny = dy * inv_len;
                let nz = dz * inv_len;

                // position on the rounded surface
                vertices.extend_from_slice(&[cx + R * nx, cy + R * ny, cz + R * nz]);
                // smooth outward normal
                vertices.extend_from_slice(&[nx, ny, nz]);
                // original face UV (same convention as the flat cube)
                vertices.extend_from_slice(&[s, 1.0 - t]);
                // original face tangent (re-orthogonalized in the vertex shader)
                vertices.extend_from_slice(&[tangent[0], tangent[1], tangent[2], 1.0]);
            }
        }

        for i in 0..subdiv {
            for j in 0..subdiv {
                let a = base + i * steps + j; // BL of this cell
                let b = a + 1; // BR
                let d = a + steps; // TL
                let c = d + 1; // TR
                indices.extend_from_slice(&[a, b, c, a, c, d]);
            }
        }
    }

    (vertices, indices)
}

/// Cylinder aligned to +Y: radius 0.7, height 2 (`y ∈ [-1, 1]`), built from a
/// side grid plus a triangle fan cap at each end.
///
/// - **Side:** `segments` around the circumference (with a duplicate seam column
///   so U can run 0→1 without wrapping) × `rings` up the axis. Normal is the
///   radial `(cosθ, 0, sinθ)`, tangent is `dP/dθ` normalized (same scheme as the
///   sphere). V is flipped (`1 - y_fraction`) so images render upright, matching
///   the plane/cube convention.
/// - **Caps:** each a fan of `segments` triangles around a center vertex. UVs are
///   a **planar projection** of the cap disc into `[0,1]²` (`u = (cosθ+1)/2`,
///   `v = (sinθ+1)/2`, center at (0.5,0.5)) — a reasonable, simple choice.
fn generate_cylinder(segments: u32, rings: u32) -> (Vec<f32>, Vec<u32>) {
    let segments = segments.max(3);
    let rings = rings.max(1);
    const RADIUS: f32 = 0.7;

    let mut vertices: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // --- Side ---
    // (segments+1) columns (seam duplicated) × (rings+1) rows of vertices.
    let cols = segments + 1;
    for j in 0..=rings {
        let fy = j as f32 / rings as f32; // 0 at bottom, 1 at top
        let y = fy * 2.0 - 1.0; // -1..1
        for i in 0..=segments {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let (sin_t, cos_t) = theta.sin_cos();

            // position on the side
            vertices.extend_from_slice(&[RADIUS * cos_t, y, RADIUS * sin_t]);
            // outward radial normal
            vertices.extend_from_slice(&[cos_t, 0.0, sin_t]);
            // uv (V flipped so texture top maps to the cylinder top)
            vertices.extend_from_slice(&[i as f32 / segments as f32, 1.0 - fy]);
            // tangent = dP/dtheta normalized; w=1 → bitangent points -Y (= +V dir)
            vertices.extend_from_slice(&[-sin_t, 0.0, cos_t, 1.0]);
        }
    }

    // Side indices, CCW from outside (verified by the outward-normal cross test).
    for j in 0..rings {
        for i in 0..segments {
            let a = j * cols + i; // BL
            let b = a + 1; // BR
            let d = a + cols; // TL
            let c = d + 1; // TR
            indices.extend_from_slice(&[a, d, c, a, c, b]);
        }
    }

    // --- Caps --- one fan per end. `push_cap` appends a center vertex followed
    // by a ring of `segments` rim vertices and the fan indices with the winding
    // required for the cap's outward normal.
    let mut push_cap = |y: f32, ny: f32, ccw: bool| {
        let center_index = (vertices.len() / VERTEX_STRIDE) as u32;
        // center
        vertices.extend_from_slice(&[0.0, y, 0.0]);
        vertices.extend_from_slice(&[0.0, ny, 0.0]);
        vertices.extend_from_slice(&[0.5, 0.5]);
        vertices.extend_from_slice(&[1.0, 0.0, 0.0, 1.0]); // tangent +X (⊥ to ±Y normal)
        // rim
        for i in 0..segments {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let (sin_t, cos_t) = theta.sin_cos();
            vertices.extend_from_slice(&[RADIUS * cos_t, y, RADIUS * sin_t]);
            vertices.extend_from_slice(&[0.0, ny, 0.0]);
            // planar UV projection of the disc into [0,1]²
            vertices.extend_from_slice(&[(cos_t + 1.0) * 0.5, (sin_t + 1.0) * 0.5]);
            vertices.extend_from_slice(&[1.0, 0.0, 0.0, 1.0]);
        }
        // fan indices
        for i in 0..segments {
            let r0 = center_index + 1 + i;
            let r1 = center_index + 1 + (i + 1) % segments;
            if ccw {
                // top cap (+Y outward): center, rim[i+1], rim[i]
                indices.extend_from_slice(&[center_index, r1, r0]);
            } else {
                // bottom cap (−Y outward): center, rim[i], rim[i+1]
                indices.extend_from_slice(&[center_index, r0, r1]);
            }
        }
    };
    push_cap(1.0, 1.0, true); // top (+Y)
    push_cap(-1.0, -1.0, false); // bottom (−Y)

    (vertices, indices)
}

/// Torus in the XZ plane: major radius `R = 0.7`, minor (tube) radius `r = 0.3`.
///
/// Standard parametric surface over `(theta, phi)`:
///   `P = ((R + r·cosφ)·cosθ, r·sinφ, (R + r·cosφ)·sinθ)`,
///   `N = (cosφ·cosθ, sinφ, cosφ·sinθ)`,
///   tangent `dP/dθ` normalized `(−sinθ, 0, cosθ)`,
///   `UV = (θ/2π, φ/2π)`.
///
/// **Fully watertight:** exactly `major_seg × minor_seg` unique vertices with the
/// index buffer wrapping modulo (`(i+1) % major_seg`, `(j+1) % minor_seg`) — no
/// duplicated seam rows — so there are no cracks even under height displacement.
/// The trade-off is a one-cell UV seam where U/V wrap 1→0 (accepted).
fn generate_torus(major_seg: u32, minor_seg: u32) -> (Vec<f32>, Vec<u32>) {
    let major_seg = major_seg.max(3);
    let minor_seg = minor_seg.max(3);
    const R: f32 = 0.7; // major radius (ring center → tube center)
    const RR: f32 = 0.3; // minor radius (tube cross-section)

    let mut vertices: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for i in 0..major_seg {
        let theta = 2.0 * std::f32::consts::PI * i as f32 / major_seg as f32;
        let (sin_t, cos_t) = theta.sin_cos();
        for j in 0..minor_seg {
            let phi = 2.0 * std::f32::consts::PI * j as f32 / minor_seg as f32;
            let (sin_p, cos_p) = phi.sin_cos();

            let ring = R + RR * cos_p;
            // position
            vertices.extend_from_slice(&[ring * cos_t, RR * sin_p, ring * sin_t]);
            // normal (unit by construction)
            vertices.extend_from_slice(&[cos_p * cos_t, sin_p, cos_p * sin_t]);
            // uv
            vertices.extend_from_slice(&[i as f32 / major_seg as f32, j as f32 / minor_seg as f32]);
            // tangent along +theta (normalized), w=1
            vertices.extend_from_slice(&[-sin_t, 0.0, cos_t, 1.0]);
        }
    }

    // Wrapped indices → shared seam vertices (watertight). Winding matches the
    // cylinder side: a, d, c / a, c, b, verified outward at (theta=0, phi=0).
    for i in 0..major_seg {
        let i_next = (i + 1) % major_seg;
        for j in 0..minor_seg {
            let j_next = (j + 1) % minor_seg;
            let a = i * minor_seg + j;
            let b = i_next * minor_seg + j;
            let c = i_next * minor_seg + j_next;
            let d = i * minor_seg + j_next;
            indices.extend_from_slice(&[a, d, c, a, c, b]);
        }
    }

    (vertices, indices)
}

/// Convert a FloatImage (1-4 channels) to RGBA f32 data.
fn to_rgba_f32(image: &FloatImage) -> Vec<f32> {
    let raw = image.as_raw();
    let channels = image.channels() as usize;
    let pixel_count = (image.width() * image.height()) as usize;
    let mut rgba = Vec::with_capacity(pixel_count * 4);

    for i in 0..pixel_count {
        let base = i * channels;
        match channels {
            1 => {
                let v = raw[base];
                rgba.extend_from_slice(&[v, v, v, 1.0]);
            }
            2 => {
                let v = raw[base];
                rgba.extend_from_slice(&[v, v, v, raw[base + 1]]);
            }
            3 => {
                rgba.extend_from_slice(&[raw[base], raw[base + 1], raw[base + 2], 1.0]);
            }
            4 => {
                rgba.extend_from_slice(&[raw[base], raw[base + 1], raw[base + 2], raw[base + 3]]);
            }
            _ => unreachable!(),
        }
    }

    rgba
}

/// Reinterpret a slice as bytes for GL buffer uploads.
fn cast_slice_to_bytes<T: Copy>(data: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            data.as_ptr() as *const u8,
            data.len() * std::mem::size_of::<T>(),
        )
    }
}

#[cfg(test)]
#[path = "gl_renderer_tests.rs"]
mod tests;
