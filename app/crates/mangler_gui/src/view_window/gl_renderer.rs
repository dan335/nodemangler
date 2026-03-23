use glow::HasContext;
use mangler_core::float_image::FloatImage;

use super::arcball_camera::ArcballCamera;

const VERTEX_SHADER: &str = include_str!("vertex.glsl");
const FRAGMENT_SHADER: &str = include_str!("fragment.glsl");

const SPHERE_SLICES: u32 = 48;
const SPHERE_STACKS: u32 = 32;

/// Number of floats per vertex: pos(3) + normal(3) + uv(2) + tangent(4) = 12
pub const VERTEX_STRIDE: usize = 12;

/// Index of each PBR texture channel (used as array index and GL texture unit).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextureChannel {
    Albedo = 0,
    Normal = 1,
    Roughness = 2,
    Metallic = 3,
    Height = 4,
    AmbientOcclusion = 5,
}

const CHANNEL_COUNT: usize = 6;

struct TextureSlot {
    texture: Option<glow::Texture>,
    change_id: Option<String>,
    u_sampler: Option<glow::UniformLocation>,
    u_has: Option<glow::UniformLocation>,
}

pub struct GlRenderer {
    program: glow::Program,
    vao: glow::VertexArray,
    _vbo: glow::Buffer,
    _ebo: glow::Buffer,
    index_count: i32,
    // Uniform locations (Option because the GLSL compiler may optimize out unused uniforms)
    u_model: Option<glow::UniformLocation>,
    u_view: Option<glow::UniformLocation>,
    u_projection: Option<glow::UniformLocation>,
    u_light_dir: Option<glow::UniformLocation>,
    u_camera_pos: Option<glow::UniformLocation>,
    // PBR texture slots
    slots: Vec<TextureSlot>,
}

impl GlRenderer {
    pub fn new(gl: &glow::Context) -> Self {
        unsafe {
            let program = create_program(gl, VERTEX_SHADER, FRAGMENT_SHADER);

            let u_model = gl.get_uniform_location(program, "u_model");
            let u_view = gl.get_uniform_location(program, "u_view");
            let u_projection = gl.get_uniform_location(program, "u_projection");
            let u_light_dir = gl.get_uniform_location(program, "u_light_dir");
            let u_camera_pos = gl.get_uniform_location(program, "u_camera_pos");

            let sampler_names = [
                "u_albedo_tex", "u_normal_tex", "u_roughness_tex",
                "u_metallic_tex", "u_height_tex", "u_ao_tex",
            ];
            let has_names = [
                "u_has_albedo", "u_has_normal", "u_has_roughness",
                "u_has_metallic", "u_has_height", "u_has_ao",
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

            let (vertices, indices) = generate_sphere(SPHERE_SLICES, SPHERE_STACKS);

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                cast_slice_to_bytes(&vertices),
                glow::STATIC_DRAW,
            );

            let ebo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                cast_slice_to_bytes(&indices),
                glow::STATIC_DRAW,
            );

            let stride = VERTEX_STRIDE as i32 * std::mem::size_of::<f32>() as i32;

            // position (location 0)
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);

            // normal (location 1)
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 3 * 4);

            // uv (location 2)
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, stride, 6 * 4);

            // tangent (location 3)
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, stride, 8 * 4);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            Self {
                program,
                vao,
                _vbo: vbo,
                _ebo: ebo,
                index_count: indices.len() as i32,
                u_model,
                u_view,
                u_projection,
                u_light_dir,
                u_camera_pos,
                slots,
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

    /// Render the scene. `viewport` is [x, y, width, height] in physical pixels.
    pub fn render(&self, gl: &glow::Context, viewport: [i32; 4], camera: &ArcballCamera) {
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

            // Light from upper-right
            let light_dir = glam::Vec3::new(0.8, 1.0, 0.6).normalize();
            gl.uniform_3_f32(self.u_light_dir.as_ref(), light_dir.x, light_dir.y, light_dir.z);

            let eye = camera.eye_position();
            gl.uniform_3_f32(self.u_camera_pos.as_ref(), eye.x, eye.y, eye.z);

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

            // Draw sphere
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_elements(glow::TRIANGLES, self.index_count, glow::UNSIGNED_INT, 0);

            // Restore state for egui's glow renderer
            gl.bind_vertex_array(None);
            gl.use_program(None);
            for i in 0..CHANNEL_COUNT {
                gl.active_texture(glow::TEXTURE0 + i as u32);
                gl.bind_texture(glow::TEXTURE_2D, None);
            }
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

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
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
