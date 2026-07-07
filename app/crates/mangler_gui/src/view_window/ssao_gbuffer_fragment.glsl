#version 330 core
// SSAO geometry pass fragment shader: writes view-space position and normal into
// the two-attachment G-buffer the SSAO pass samples. No lighting here — this is
// purely a data pass. RGBA16F attachments; the alpha channels are unused (1.0).
in vec3 v_view_pos;
in vec3 v_view_normal;

layout(location = 0) out vec4 g_position;
layout(location = 1) out vec4 g_normal;

void main() {
    g_position = vec4(v_view_pos, 1.0);
    g_normal = vec4(normalize(v_view_normal), 1.0);
}
