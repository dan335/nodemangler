#version 330 core
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_uv;
layout(location = 3) in vec4 a_tangent;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;

// Height displacement (Phase 2). The height texture is shared with the fragment
// shader (same program, same uniform location / texture unit). `u_has_height`
// is 0 when no Height channel is bound; `u_height_scale` is the displacement
// amount along the surface normal (meshes span ~2 world units).
uniform sampler2D u_height_tex;
uniform int u_has_height;
uniform float u_height_scale;

// UV tiling multiplier (Phase 4). The material textures repeat `u_uv_tiling`
// times across the mesh; the tiled UV is used both for the height displacement
// sample here and passed to the fragment shader as v_uv. Values >1 on the
// sphere/cylinder wrap through the UV seam (expected).
uniform float u_uv_tiling;

out vec3 v_world_pos;
out vec3 v_normal;
out vec2 v_uv;
out mat3 v_tbn;

void main() {
    // Displace the base position along its normal by the (centered) height.
    // 0.5 is the neutral height level, so a flat 0.5 map leaves the mesh
    // unchanged. `textureLod` with LOD 0 is required here: the vertex shader has
    // no fragment derivatives, so a plain `texture()` mip selection is undefined.
    // Tiled UV, shared by the height sample and the fragment shader.
    vec2 uv = a_uv * u_uv_tiling;

    vec3 pos = a_position;
    if (u_has_height != 0) {
        float h = textureLod(u_height_tex, uv, 0.0).r;
        pos += a_normal * (h - 0.5) * u_height_scale;
    }

    vec4 world = u_model * vec4(pos, 1.0);
    v_world_pos = world.xyz;

    mat3 normal_matrix = mat3(transpose(inverse(u_model)));
    vec3 N = normalize(normal_matrix * a_normal);
    vec3 T = normalize(normal_matrix * a_tangent.xyz);
    T = normalize(T - dot(T, N) * N); // re-orthogonalize
    vec3 B = cross(N, T) * a_tangent.w;
    v_tbn = mat3(T, B, N);

    v_normal = N;
    v_uv = uv;
    gl_Position = u_projection * u_view * world;
}
