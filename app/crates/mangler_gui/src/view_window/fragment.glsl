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
uniform sampler2D u_emissive_tex;

// Flags (0 = no texture, use default)
uniform int u_has_albedo;
uniform int u_has_normal;
uniform int u_has_roughness;
uniform int u_has_metallic;
uniform int u_has_height;
uniform int u_has_ao;
uniform int u_has_emissive;

uniform vec3 u_light_dir;
uniform vec3 u_light_color; // light radiance (color * intensity)
uniform vec3 u_camera_pos;

// CPU-prefiltered environment LUTs (see environment.rs). The procedural sky is
// yaw-symmetric, so diffuse irradiance depends only on N.y (64×1) and
// prefiltered specular only on (R.y, roughness) (64×64).
uniform sampler2D u_irradiance_lut;
uniform sampler2D u_specular_lut;
// Multiplier on the environment (IBL) contribution; 0 = no ambient.
uniform float u_env_intensity;

// Height displacement scale, shared with the vertex shader (same program).
// Used here to reconstruct a perturbed normal from the height field when only
// a Height channel is bound (no Normal channel).
uniform float u_height_scale;

// Tone-map selector (matches ToneMap::to_int), applied before the gamma step.
uniform int u_tone_map;

// Wireframe overlay pass: when u_use_flat_color != 0 the shader short-circuits
// and outputs u_flat_color (a theme color), so the overlay draw is a flat line
// color rather than a re-shade of the surface.
uniform int u_use_flat_color;
uniform vec4 u_flat_color;

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

// Shared tone-map GLSL (apply_tone_map / hable_partial) is spliced in here at
// load time by mesh_fragment_source() — the same definition the sky shader uses.
//__TONE_MAP_GLSL__

void main() {
    // Wireframe overlay pass: emit the flat line color and skip all shading.
    if (u_use_flat_color != 0) {
        frag_color = u_flat_color;
        return;
    }

    // Sample material properties
    vec3 albedo = u_has_albedo != 0 ? pow(texture(u_albedo_tex, v_uv).rgb, vec3(2.2)) : vec3(0.5);
    float roughness = u_has_roughness != 0 ? texture(u_roughness_tex, v_uv).r : 0.5;
    float metallic = u_has_metallic != 0 ? texture(u_metallic_tex, v_uv).r : 0.0;
    float ao = u_has_ao != 0 ? texture(u_ao_tex, v_uv).r : 1.0;

    // Normal selection (Substance-style priority):
    //   1. Normal channel bound      → use it directly through the TBN.
    //   2. Only Height bound         → reconstruct a normal from the height
    //                                   field via central differences.
    //   3. Neither                   → interpolated geometric normal.
    vec3 N;
    if (u_has_normal != 0) {
        vec3 normal_sample = texture(u_normal_tex, v_uv).rgb * 2.0 - 1.0;
        N = normalize(v_tbn * normal_sample);
    } else if (u_has_height != 0) {
        // Central differences of the height field, one texel apart.
        vec2 texel = 1.0 / vec2(textureSize(u_height_tex, 0));
        float hl = texture(u_height_tex, v_uv - vec2(texel.x, 0.0)).r;
        float hr = texture(u_height_tex, v_uv + vec2(texel.x, 0.0)).r;
        float hd = texture(u_height_tex, v_uv - vec2(0.0, texel.y)).r;
        float hu = texture(u_height_tex, v_uv + vec2(0.0, texel.y)).r;

        // WORLD_SIZE is the world-space span of one UV tile (meshes are ~2 units
        // across per full UV range). It relates a UV-space height slope to a
        // world-space slope; treat it as an empirically-tuned Z-scale factor —
        // it, together with u_height_scale, sets how "steep" the reconstructed
        // normals look and is the value to adjust if displacement lighting reads
        // too flat or too harsh (and it will need revisiting once UV tiling
        // lands in a later phase).
        const float WORLD_SIZE = 2.0;

        // Tangent-space normal. The X component (along the tangent T = +U) is the
        // standard downhill-tilt: nx < 0 where height rises with +U. The Y
        // component (along the bitangent B) accounts for this mesh's FLIPPED V:
        // the vertex-shader TBN yields B = +Y while UV V increases downward
        // (v = (1-y)/2 on the plane), i.e. B points along -V. Working the surface
        // cross-product through that basis gives ny = +slope·(dh/dv), so we use
        // (hu - hd) (NOT hd - hu). Verified by deriving cross(dS/du, dS/dv) in the
        // TBN basis; get this sign wrong and height relief lights inverted along V.
        float slope_x = u_height_scale * (hl - hr) / (2.0 * texel.x * WORLD_SIZE);
        float slope_y = u_height_scale * (hu - hd) / (2.0 * texel.y * WORLD_SIZE);
        vec3 ts_normal = normalize(vec3(slope_x, slope_y, 1.0));
        N = normalize(v_tbn * ts_normal);
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
    vec3 Lo = (kD * albedo / PI + specular) * u_light_color * NdotL; // radiance from uniform

    // Ambient: image-based lighting from the CPU-prefiltered sky LUTs.
    // The sky is yaw-symmetric, so N.y / R.y fully index the convolutions.
    vec3 R = reflect(-V, N);
    vec3 irradiance = texture(u_irradiance_lut, vec2(N.y * 0.5 + 0.5, 0.5)).rgb;
    vec3 prefiltered = texture(u_specular_lut, vec2(R.y * 0.5 + 0.5, roughness)).rgb;
    // Karis analytic split-sum environment BRDF approximation (no LUT texture needed)
    vec4 c0 = vec4(-1.0, -0.0275, -0.572, 0.022);
    vec4 c1 = vec4( 1.0,  0.0425,  1.04, -0.04);
    vec4 r4 = roughness * c0 + c1;
    float a004 = min(r4.x * r4.x, exp2(-9.28 * max(dot(N, V), 0.0))) * r4.x + r4.y;
    vec2 ab = vec2(-1.04, 1.04) * a004 + r4.zw;
    vec3 kD_ambient = (1.0 - fresnel_schlick_roughness(max(dot(N, V), 0.0), F0, roughness)) * (1.0 - metallic);
    vec3 ambient = (kD_ambient * albedo * irradiance + prefiltered * (F0 * ab.x + ab.y)) * ao * u_env_intensity;

    // Emissive: self-illumination added on top of the lit result, PRE-tonemap
    // (de-gamma'd from sRGB like albedo). No texture → no emission.
    vec3 emissive = u_has_emissive != 0
        ? pow(texture(u_emissive_tex, v_uv).rgb, vec3(2.2))
        : vec3(0.0);

    vec3 color = ambient + Lo + emissive;

    // Selectable tone map (shared with the sky shader), then the separate gamma
    // step that runs for every mode.
    color = apply_tone_map(color, u_tone_map);
    color = pow(color, vec3(1.0 / 2.2));

    frag_color = vec4(color, 1.0);
}
