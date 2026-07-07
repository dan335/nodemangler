#version 330 core
// SSAO blur pass: a 4x4 box blur that removes the noise pattern the per-pixel
// kernel rotation bakes into the raw SSAO term. The 4x4 window exactly matches
// the 4x4 noise tile, so averaging over it cancels the rotation dither and
// yields a smooth occlusion field. Reads and writes single-channel occlusion in
// the .r of an RGBA16F target.
in vec2 v_ndc; // fullscreen-triangle clip xy in [-1,1] (from the shared sky VS)

uniform sampler2D u_ssao; // raw occlusion from ssao_fragment.glsl

out vec4 frag_ao;

void main() {
    vec2 uv = v_ndc * 0.5 + 0.5;
    vec2 texel = 1.0 / vec2(textureSize(u_ssao, 0));

    float sum = 0.0;
    // Offsets -2..+1 centre the 4x4 window on the current texel.
    for (int x = -2; x < 2; ++x) {
        for (int y = -2; y < 2; ++y) {
            sum += texture(u_ssao, uv + vec2(float(x), float(y)) * texel).r;
        }
    }

    float blurred = sum / 16.0;
    frag_ao = vec4(blurred, blurred, blurred, 1.0);
}
