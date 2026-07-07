#version 330 core
// SSAO geometry pass vertex shader. Emits VIEW-space position and normal for
// every mesh fragment so the SSAO fragment pass can sample a hemisphere of
// neighbouring depths. It must apply the SAME height displacement as the main
// `vertex.glsl` (identical `u_has_height` / `u_height_scale` / `u_uv_tiling`
// handling) so the ambient occlusion lines up with the displaced silhouette the
// user actually sees — get the displacement out of sync and the AO "floats".
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_uv;
layout(location = 3) in vec4 a_tangent;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;

// Height displacement, matching vertex.glsl exactly (same program-independent
// uniform names, bound to the same values by the renderer).
uniform sampler2D u_height_tex;
uniform int u_has_height;
uniform float u_height_scale;
uniform float u_uv_tiling;

out vec3 v_view_pos;    // fragment position in view space
out vec3 v_view_normal; // geometric normal in view space

void main() {
    // Same tiled UV + centred height displacement as the main vertex shader.
    vec2 uv = a_uv * u_uv_tiling;
    vec3 pos = a_position;
    if (u_has_height != 0) {
        float h = textureLod(u_height_tex, uv, 0.0).r;
        pos += a_normal * (h - 0.5) * u_height_scale;
    }

    vec4 world = u_model * vec4(pos, 1.0);
    vec4 view_pos = u_view * world;
    v_view_pos = view_pos.xyz;

    // model is identity and the view matrix is a rigid (rotation+translation)
    // transform, so mat3(u_view) is orthonormal and can transform the normal
    // directly — no inverse-transpose needed. (Kept simple deliberately: SSAO is
    // a coarse geometric effect and does not use the fine normal-map detail.)
    v_view_normal = normalize(mat3(u_view) * a_normal);

    gl_Position = u_projection * view_pos;
}
