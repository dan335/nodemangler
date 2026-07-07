#version 330 core

// Depth-only fragment shader for the directional shadow map caster pass.
//
// The shadow pass renders the scene from the light's point of view purely to
// populate the depth buffer, so this shader writes nothing — `gl_FragDepth` is
// left to the fixed-function default (the interpolated window-space depth).
//
// It is paired with the UNCHANGED `vertex.glsl` (reused as the caster VS) so
// the height-displaced geometry the light "sees" matches the main pass exactly
// — that self-consistency is what lets displaced surfaces self-shadow. All of
// vertex.glsl's `out` varyings (v_world_pos, v_normal, v_uv, v_tbn) are simply
// unused here, which is legal GLSL: an unread varying just gets discarded.
void main() {}
