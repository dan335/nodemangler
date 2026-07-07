#version 330 core
// Screen-space ambient occlusion, Crytek/LearnOpenGL-style hemisphere sampling.
//
// For each pixel we read its view-space position + normal from the G-buffer,
// build a TBN oriented by the surface normal and jittered by a tiled noise
// texture (rotates the kernel per-pixel so 32 samples look like far more after
// the blur pass), then march a hemisphere of sample points, project each back to
// screen space and compare its depth against the stored geometry depth. Samples
// that land *behind* nearer geometry count as occluders. The result is a single
// [0,1] visibility term (1 = open, 0 = fully occluded) written to a float target,
// then smoothed by ssao_blur.glsl before the main pass multiplies it into ambient.
//
// KERNEL_SIZE must stay in sync with SSAO_KERNEL_SIZE in gl_renderer.rs (the CPU
// generates exactly this many hemisphere samples and uploads them to u_kernel).
#define KERNEL_SIZE 32

in vec2 v_ndc; // fullscreen-triangle clip xy in [-1,1] (from the shared sky VS)

uniform sampler2D u_g_position; // view-space position (RGBA16F, .xyz)
uniform sampler2D u_g_normal;   // view-space normal   (RGBA16F, .xyz)
uniform sampler2D u_noise;      // 4x4 tiled rotation vectors (RGBA16F, .xy)

uniform vec3 u_kernel[KERNEL_SIZE]; // hemisphere sample offsets (tangent space)
uniform mat4 u_projection;          // same projection as the main/gbuffer pass
uniform vec2 u_noise_scale;         // viewport / 4, so the 4x4 noise tiles 1:1
uniform float u_radius;             // sampling hemisphere radius, view-space units
uniform float u_bias;               // depth bias to avoid self-occlusion acne

out vec4 frag_ao; // occlusion written to .r (RGBA16F target)

void main() {
    vec2 uv = v_ndc * 0.5 + 0.5;

    vec3 frag_pos = texture(u_g_position, uv).xyz;
    vec3 normal = normalize(texture(u_g_normal, uv).xyz);
    // Background texels are cleared to 0 (zero-length normal): nothing to occlude,
    // and the main pass never samples AO off the mesh, so emit fully-open.
    if (dot(normal, normal) < 0.5) {
        frag_ao = vec4(1.0);
        return;
    }

    // Per-pixel random rotation of the kernel about the normal (Gram-Schmidt).
    vec3 random_vec = normalize(texture(u_noise, uv * u_noise_scale).xyz);
    vec3 tangent = normalize(random_vec - normal * dot(random_vec, normal));
    vec3 bitangent = cross(normal, tangent);
    mat3 tbn = mat3(tangent, bitangent, normal);

    float occlusion = 0.0;
    for (int i = 0; i < KERNEL_SIZE; ++i) {
        // Kernel sample → view space, offset from the fragment by the radius.
        vec3 sample_pos = frag_pos + (tbn * u_kernel[i]) * u_radius;

        // Project the sample into screen space to look up the stored depth there.
        vec4 offset = u_projection * vec4(sample_pos, 1.0);
        offset.xyz /= offset.w;
        offset.xyz = offset.xyz * 0.5 + 0.5;
        // Off-screen samples have no depth to compare against — skip them.
        if (offset.x < 0.0 || offset.x > 1.0 || offset.y < 0.0 || offset.y > 1.0) {
            continue;
        }

        float sample_depth = texture(u_g_position, offset.xy).z;
        // Range check: ignore occluders far outside the radius (a distant wall
        // behind a thin object must not darken it). smoothstep fades their weight.
        float range_check = smoothstep(0.0, 1.0, u_radius / max(abs(frag_pos.z - sample_depth), 0.0001));
        // View space looks down -Z: a larger (less negative) sample_depth means
        // the stored geometry is nearer the camera than our sample → occluder.
        occlusion += (sample_depth >= sample_pos.z + u_bias ? 1.0 : 0.0) * range_check;
    }

    float visibility = 1.0 - occlusion / float(KERNEL_SIZE);
    frag_ao = vec4(visibility, visibility, visibility, 1.0);
}
