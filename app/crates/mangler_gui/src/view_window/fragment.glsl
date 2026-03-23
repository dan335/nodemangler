#version 330 core
const float PI = 3.14159265359;

in vec3 v_world_pos;
in vec3 v_normal;
in vec2 v_uv;
in mat3 v_tbn;

// PBR textures
uniform sampler2D u_albedo_tex;
uniform sampler2D u_normal_tex;
uniform sampler2D u_roughness_tex;
uniform sampler2D u_metallic_tex;
uniform sampler2D u_height_tex;
uniform sampler2D u_ao_tex;

// Flags (0 = no texture, use default)
uniform int u_has_albedo;
uniform int u_has_normal;
uniform int u_has_roughness;
uniform int u_has_metallic;
uniform int u_has_height;
uniform int u_has_ao;

uniform vec3 u_light_dir;
uniform vec3 u_camera_pos;

out vec4 frag_color;

// --- PBR functions ---

// GGX/Trowbridge-Reitz normal distribution
float distribution_ggx(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;
    float denom = NdotH2 * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom);
}

// Schlick-GGX geometry function
float geometry_schlick_ggx(float NdotV, float roughness) {
    float r = roughness + 1.0;
    float k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k);
}

// Smith's geometry function
float geometry_smith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    return geometry_schlick_ggx(NdotV, roughness) * geometry_schlick_ggx(NdotL, roughness);
}

// Fresnel-Schlick approximation
vec3 fresnel_schlick(float cos_theta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// Fresnel-Schlick with roughness for ambient
vec3 fresnel_schlick_roughness(float cos_theta, vec3 F0, float roughness) {
    return F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

void main() {
    // Sample material properties
    vec3 albedo = u_has_albedo != 0 ? pow(texture(u_albedo_tex, v_uv).rgb, vec3(2.2)) : vec3(0.5);
    float roughness = u_has_roughness != 0 ? texture(u_roughness_tex, v_uv).r : 0.5;
    float metallic = u_has_metallic != 0 ? texture(u_metallic_tex, v_uv).r : 0.0;
    float ao = u_has_ao != 0 ? texture(u_ao_tex, v_uv).r : 1.0;

    // Normal mapping
    vec3 N;
    if (u_has_normal != 0) {
        vec3 normal_sample = texture(u_normal_tex, v_uv).rgb * 2.0 - 1.0;
        N = normalize(v_tbn * normal_sample);
    } else {
        N = normalize(v_normal);
    }

    vec3 V = normalize(u_camera_pos - v_world_pos);
    vec3 L = normalize(u_light_dir);
    vec3 H = normalize(V + L);

    // F0 = reflectance at normal incidence
    vec3 F0 = mix(vec3(0.04), albedo, metallic);

    // Cook-Torrance BRDF
    float NDF = distribution_ggx(N, H, roughness);
    float G = geometry_smith(N, V, L, roughness);
    vec3 F = fresnel_schlick(max(dot(H, V), 0.0), F0);

    vec3 numerator = NDF * G * F;
    float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
    vec3 specular = numerator / denominator;

    vec3 kS = F;
    vec3 kD = (1.0 - kS) * (1.0 - metallic);

    float NdotL = max(dot(N, L), 0.0);
    vec3 Lo = (kD * albedo / PI + specular) * vec3(3.0) * NdotL; // light radiance = 3.0

    // Ambient: analytical sky gradient
    float up = N.y * 0.5 + 0.5;
    vec3 sky_color = mix(vec3(0.15, 0.12, 0.1), vec3(0.3, 0.4, 0.6), up);
    vec3 F_ambient = fresnel_schlick_roughness(max(dot(N, V), 0.0), F0, roughness);
    vec3 kD_ambient = (1.0 - F_ambient) * (1.0 - metallic);
    vec3 ambient = (kD_ambient * albedo * sky_color + F_ambient * sky_color * 0.3) * ao;

    vec3 color = ambient + Lo;

    // Reinhard tonemap
    color = color / (color + vec3(1.0));
    // Gamma correction
    color = pow(color, vec3(1.0 / 2.2));

    frag_color = vec4(color, 1.0);
}
