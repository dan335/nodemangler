#version 330 core
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_uv;
layout(location = 3) in vec4 a_tangent;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;

out vec3 v_world_pos;
out vec3 v_normal;
out vec2 v_uv;
out mat3 v_tbn;

void main() {
    vec4 world = u_model * vec4(a_position, 1.0);
    v_world_pos = world.xyz;

    mat3 normal_matrix = mat3(transpose(inverse(u_model)));
    vec3 N = normalize(normal_matrix * a_normal);
    vec3 T = normalize(normal_matrix * a_tangent.xyz);
    T = normalize(T - dot(T, N) * N); // re-orthogonalize
    vec3 B = cross(N, T) * a_tangent.w;
    v_tbn = mat3(T, B, N);

    v_normal = N;
    v_uv = a_uv;
    gl_Position = u_projection * u_view * world;
}
