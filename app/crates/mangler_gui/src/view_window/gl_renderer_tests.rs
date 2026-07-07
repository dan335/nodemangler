use super::*;

#[test]
fn sphere_vertex_count() {
    let (vertices, _indices) = generate_sphere(16, 8);
    let expected_vertices = (16 + 1) * (8 + 1);
    assert_eq!(vertices.len(), expected_vertices * VERTEX_STRIDE);
}

#[test]
fn sphere_index_count() {
    let (_vertices, indices) = generate_sphere(16, 8);
    let expected_triangles = 16 * 8 * 2;
    assert_eq!(indices.len(), expected_triangles * 3);
}

#[test]
fn sphere_indices_in_bounds() {
    let (vertices, indices) = generate_sphere(32, 16);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for &idx in &indices {
        assert!(
            (idx as usize) < vertex_count,
            "Index {} out of bounds (vertex count: {})",
            idx,
            vertex_count
        );
    }
}

#[test]
fn sphere_normals_are_unit_length() {
    let (vertices, _) = generate_sphere(16, 8);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let nx = vertices[base + 3];
        let ny = vertices[base + 4];
        let nz = vertices[base + 5];
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-4,
            "Normal at vertex {} has length {} (expected 1.0)",
            i, len
        );
    }
}

#[test]
fn sphere_uvs_in_range() {
    let (vertices, _) = generate_sphere(32, 16);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let u = vertices[base + 6];
        let v = vertices[base + 7];
        assert!(u >= 0.0 && u <= 1.0, "UV u={} out of range at vertex {}", u, i);
        assert!(v >= 0.0 && v <= 1.0, "UV v={} out of range at vertex {}", v, i);
    }
}

#[test]
fn sphere_positions_on_unit_sphere() {
    let (vertices, _) = generate_sphere(24, 12);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let x = vertices[base];
        let y = vertices[base + 1];
        let z = vertices[base + 2];
        let r = (x * x + y * y + z * z).sqrt();
        assert!(
            (r - 1.0).abs() < 1e-4,
            "Position at vertex {} has radius {} (expected 1.0)",
            i, r
        );
    }
}

#[test]
fn sphere_tangents_are_unit_length() {
    let (vertices, _) = generate_sphere(24, 12);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let tx = vertices[base + 8];
        let ty = vertices[base + 9];
        let tz = vertices[base + 10];
        let len = (tx * tx + ty * ty + tz * tz).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-4,
            "Tangent at vertex {} has length {} (expected 1.0)",
            i, len
        );
    }
}

#[test]
fn sphere_tangents_perpendicular_to_normals() {
    let (vertices, _) = generate_sphere(24, 12);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let nx = vertices[base + 3];
        let ny = vertices[base + 4];
        let nz = vertices[base + 5];
        let tx = vertices[base + 8];
        let ty = vertices[base + 9];
        let tz = vertices[base + 10];
        let dot = nx * tx + ny * ty + nz * tz;
        // At poles the tangent is arbitrary, allow some slack
        assert!(
            dot.abs() < 0.1,
            "Tangent not perpendicular to normal at vertex {} (dot={})",
            i, dot
        );
    }
}

#[test]
fn sphere_bitangent_sign_is_positive() {
    let (vertices, _) = generate_sphere(16, 8);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let tw = vertices[base + 11];
        assert_eq!(tw, 1.0, "Bitangent sign should be 1.0 at vertex {}", i);
    }
}

// --- Plane mesh (parametric grid) ---

#[test]
fn plane_vertex_count() {
    // (subdiv+1)² vertices.
    let (vertices, _) = generate_plane(4);
    let expected_vertices = (4 + 1) * (4 + 1);
    assert_eq!(vertices.len(), expected_vertices * VERTEX_STRIDE);
}

#[test]
fn plane_index_count() {
    // subdiv² quads, 2 triangles each, 3 indices each.
    let (_, indices) = generate_plane(4);
    let expected_triangles = 4 * 4 * 2;
    assert_eq!(indices.len(), expected_triangles * 3);
}

#[test]
fn plane_indices_in_bounds() {
    let (vertices, indices) = generate_plane(8);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for &idx in &indices {
        assert!(
            (idx as usize) < vertex_count,
            "Index {} out of bounds (vertex count: {})",
            idx,
            vertex_count
        );
    }
}

#[test]
fn plane_normals_are_unit_length_and_facing_z() {
    let (vertices, _) = generate_plane(4);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let (nx, ny, nz) = (vertices[base + 3], vertices[base + 4], vertices[base + 5]);
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!((len - 1.0).abs() < 1e-6, "Plane normal not unit length");
        // Whole plane faces +Z.
        assert!((nx).abs() < 1e-6 && (ny).abs() < 1e-6 && (nz - 1.0).abs() < 1e-6);
    }
}

#[test]
fn plane_uvs_in_range() {
    let (vertices, _) = generate_plane(8);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let (u, v) = (vertices[base + 6], vertices[base + 7]);
        assert!(u >= 0.0 && u <= 1.0, "Plane u={} out of range", u);
        assert!(v >= 0.0 && v <= 1.0, "Plane v={} out of range", v);
    }
}

#[test]
fn plane_v_flip_preserved() {
    // The original 4-vertex quad assigned uv (0,0) to the TOP-LEFT corner
    // (−1,+1) and uv (0,1) to the BOTTOM-LEFT corner (−1,−1), so the image
    // renders right-side-up despite OpenGL sampling v=0 from the first row.
    // The tessellated grid must keep this exact convention.
    let (vertices, _) = generate_plane(4);
    let vertex_count = vertices.len() / VERTEX_STRIDE;

    let mut top_left_uv = None;
    let mut bottom_left_uv = None;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let (x, y) = (vertices[base], vertices[base + 1]);
        let (u, v) = (vertices[base + 6], vertices[base + 7]);
        if (x + 1.0).abs() < 1e-6 && (y - 1.0).abs() < 1e-6 {
            top_left_uv = Some((u, v));
        }
        if (x + 1.0).abs() < 1e-6 && (y + 1.0).abs() < 1e-6 {
            bottom_left_uv = Some((u, v));
        }
    }
    assert_eq!(top_left_uv, Some((0.0, 0.0)), "TL corner uv should be (0,0)");
    assert_eq!(
        bottom_left_uv,
        Some((0.0, 1.0)),
        "BL corner uv should be (0,1)"
    );
}

// --- Cube mesh (per-face parametric grids) ---

#[test]
fn cube_vertex_count() {
    // 6 faces, each a (subdiv+1)² grid (faces kept separate for seams).
    let (vertices, _) = generate_cube(2);
    let expected_vertices = 6 * (2 + 1) * (2 + 1);
    assert_eq!(vertices.len(), expected_vertices * VERTEX_STRIDE);
}

#[test]
fn cube_index_count() {
    // 6 faces × subdiv² quads × 2 triangles × 3 indices.
    let (_, indices) = generate_cube(2);
    let expected_triangles = 6 * 2 * 2 * 2;
    assert_eq!(indices.len(), expected_triangles * 3);
}

#[test]
fn cube_indices_in_bounds() {
    let (vertices, indices) = generate_cube(4);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for &idx in &indices {
        assert!(
            (idx as usize) < vertex_count,
            "Index {} out of bounds (vertex count: {})",
            idx,
            vertex_count
        );
    }
}

#[test]
fn cube_normals_are_unit_length() {
    let (vertices, _) = generate_cube(3);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let (nx, ny, nz) = (vertices[base + 3], vertices[base + 4], vertices[base + 5]);
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!((len - 1.0).abs() < 1e-6, "Cube normal not unit length");
    }
}

#[test]
fn cube_uvs_in_range() {
    let (vertices, _) = generate_cube(4);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let (u, v) = (vertices[base + 6], vertices[base + 7]);
        assert!(u >= 0.0 && u <= 1.0, "Cube u={} out of range", u);
        assert!(v >= 0.0 && v <= 1.0, "Cube v={} out of range", v);
    }
}

#[test]
fn cube_positions_within_extents() {
    // Every cube vertex lies within (actually on the surface of) ±1 extents.
    let (vertices, _) = generate_cube(3);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        for k in 0..3 {
            let c = vertices[base + k];
            assert!(c >= -1.0 - 1e-6 && c <= 1.0 + 1e-6, "Cube coord {} out of extents", c);
        }
    }
}

// --- Mesh resolution enum ---

#[test]
fn mesh_resolution_all_and_labels() {
    // ALL lists every variant once, coarse → fine, with distinct labels.
    assert_eq!(MeshResolution::ALL.len(), 3);
    assert_eq!(MeshResolution::Low.label(), "Low");
    assert_eq!(MeshResolution::Medium.label(), "Medium");
    assert_eq!(MeshResolution::High.label(), "High");
    assert_eq!(MeshResolution::default(), MeshResolution::Medium);

    // Subdivision counts increase monotonically with resolution.
    let subdivs: Vec<u32> = MeshResolution::ALL.iter().map(|r| r.plane_subdiv()).collect();
    assert!(subdivs[0] < subdivs[1] && subdivs[1] < subdivs[2]);
    let cube: Vec<u32> = MeshResolution::ALL.iter().map(|r| r.cube_subdiv()).collect();
    assert!(cube[0] < cube[1] && cube[1] < cube[2]);
    for r in MeshResolution::ALL {
        let (slices, stacks) = r.sphere_slices_stacks();
        assert!(slices > 0 && stacks > 0);
        let (segments, rings) = r.cylinder_segments_rings();
        assert!(segments > 0 && rings > 0);
        let (major, minor) = r.torus_major_minor();
        assert!(major > 0 && minor > 0);
    }
    // Cylinder/torus resolutions also increase monotonically.
    let cyl: Vec<u32> = MeshResolution::ALL.iter().map(|r| r.cylinder_segments_rings().0).collect();
    assert!(cyl[0] < cyl[1] && cyl[1] < cyl[2]);
    let tor: Vec<u32> = MeshResolution::ALL.iter().map(|r| r.torus_major_minor().0).collect();
    assert!(tor[0] < tor[1] && tor[1] < tor[2]);
}

#[test]
fn to_rgba_f32_grayscale() {
    let img = FloatImage::from_pixel(2, 2, 1, &[0.5]);
    let rgba = to_rgba_f32(&img);
    assert_eq!(rgba.len(), 2 * 2 * 4);
    for i in 0..4 {
        let base = i * 4;
        assert_eq!(rgba[base], 0.5);
        assert_eq!(rgba[base + 1], 0.5);
        assert_eq!(rgba[base + 2], 0.5);
        assert_eq!(rgba[base + 3], 1.0);
    }
}

#[test]
fn to_rgba_f32_rgb() {
    let img = FloatImage::from_pixel(1, 1, 3, &[0.1, 0.2, 0.3]);
    let rgba = to_rgba_f32(&img);
    assert_eq!(rgba.len(), 4);
    assert!((rgba[0] - 0.1).abs() < 1e-6);
    assert!((rgba[1] - 0.2).abs() < 1e-6);
    assert!((rgba[2] - 0.3).abs() < 1e-6);
    assert_eq!(rgba[3], 1.0);
}

#[test]
fn to_rgba_f32_rgba_passthrough() {
    let img = FloatImage::from_pixel(1, 1, 4, &[0.1, 0.2, 0.3, 0.4]);
    let rgba = to_rgba_f32(&img);
    assert_eq!(rgba.len(), 4);
    assert!((rgba[0] - 0.1).abs() < 1e-6);
    assert!((rgba[1] - 0.2).abs() < 1e-6);
    assert!((rgba[2] - 0.3).abs() < 1e-6);
    assert!((rgba[3] - 0.4).abs() < 1e-6);
}

#[test]
fn to_rgba_f32_two_channel() {
    let img = FloatImage::from_pixel(1, 1, 2, &[0.7, 0.3]);
    let rgba = to_rgba_f32(&img);
    assert_eq!(rgba.len(), 4);
    assert!((rgba[0] - 0.7).abs() < 1e-6);
    assert!((rgba[1] - 0.7).abs() < 1e-6);
    assert!((rgba[2] - 0.7).abs() < 1e-6);
    assert!((rgba[3] - 0.3).abs() < 1e-6);
}

// --- Sky shader GLSL (generated from environment.rs constants) ---

#[test]
fn sky_glsl_matches_environment_constants() {
    // The sky GLSL is built via format!() from environment.rs's pub constants,
    // so the two *cannot* drift; this test locks that mechanism in by checking
    // the generated source embeds exactly those constant values.
    let glsl = sky_glsl();
    for (name, c) in [
        ("SKY_ZENITH", ZENITH),
        ("SKY_HORIZON", HORIZON),
        ("SKY_GROUND", GROUND),
    ] {
        let expected = format!("const vec3 {} = vec3({:?}, {:?}, {:?});", name, c.x, c.y, c.z);
        assert!(
            glsl.contains(&expected),
            "generated sky GLSL missing `{}`:\n{}",
            expected,
            glsl
        );
    }
}

#[test]
fn sky_fragment_source_is_complete() {
    // The assembled sky fragment shader must contain the shared sky gradient,
    // the shared tone map, the sun disc, and the uniforms the renderer sets.
    let src = sky_fragment_source();
    for needle in [
        "vec3 sky_radiance(vec3 dir)",
        "apply_tone_map",
        "u_inv_view_proj",
        "u_light_dir",
        "u_light_color",
        "u_tone_map",
        "#version 330 core",
    ] {
        assert!(src.contains(needle), "sky fragment source missing `{}`", needle);
    }
    // No un-substituted format placeholders left behind.
    assert!(!src.contains("{sky}") && !src.contains("{tone_map}"));
}

#[test]
fn mesh_fragment_source_splices_tone_map() {
    // The mesh fragment shader is assembled by replacing the placeholder token
    // with the shared tone-map GLSL. After assembly the shared function must be
    // present and the placeholder gone.
    let src = mesh_fragment_source();
    assert!(
        src.contains("vec3 apply_tone_map(vec3 color, int mode)"),
        "mesh fragment source missing shared apply_tone_map"
    );
    assert!(
        !src.contains(TONE_MAP_PLACEHOLDER),
        "mesh fragment source still contains the unreplaced placeholder token"
    );
    // The emissive + wireframe uniforms the Phase 4 shader relies on.
    for needle in ["u_emissive_tex", "u_has_emissive", "u_tone_map", "u_use_flat_color", "u_flat_color"] {
        assert!(src.contains(needle), "mesh fragment source missing `{}`", needle);
    }
}

#[test]
fn both_fragment_sources_share_apply_tone_map() {
    // Mesh and sky must tonemap through the SAME function so they never drift.
    assert!(mesh_fragment_source().contains("apply_tone_map"));
    assert!(sky_fragment_source().contains("apply_tone_map"));
}

// --- Cylinder mesh ---

#[test]
fn cylinder_counts_and_bounds() {
    let (segments, rings) = (16u32, 4u32);
    let (vertices, indices) = generate_cylinder(segments, rings);
    let vertex_count = vertices.len() / VERTEX_STRIDE;

    // Side: (segments+1)*(rings+1). Caps: 2 * (1 center + segments rim).
    let expected_vertices =
        (segments + 1) * (rings + 1) + 2 * (1 + segments);
    assert_eq!(vertex_count, expected_vertices as usize);

    // Side: segments*rings quads * 2 tris. Caps: 2 * segments fan tris.
    let expected_tris = segments * rings * 2 + 2 * segments;
    assert_eq!(indices.len(), expected_tris as usize * 3);

    for &idx in &indices {
        assert!((idx as usize) < vertex_count, "cylinder index {} OOB", idx);
    }
}

#[test]
fn cylinder_normals_unit_and_positions_bounded() {
    let (vertices, _) = generate_cylinder(24, 6);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let (nx, ny, nz) = (vertices[base + 3], vertices[base + 4], vertices[base + 5]);
        assert!(((nx * nx + ny * ny + nz * nz).sqrt() - 1.0).abs() < 1e-4, "cylinder normal not unit");
        // radius 0.7, height 2 (y in [-1,1]); x/z within ±0.7.
        let (x, y, z) = (vertices[base], vertices[base + 1], vertices[base + 2]);
        assert!(y >= -1.0 - 1e-6 && y <= 1.0 + 1e-6, "cylinder y {} out of range", y);
        assert!(x.abs() <= 0.7 + 1e-6 && z.abs() <= 0.7 + 1e-6, "cylinder x/z out of radius");
    }
}

#[test]
fn cylinder_uvs_in_range() {
    let (vertices, _) = generate_cylinder(20, 5);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let (u, v) = (vertices[base + 6], vertices[base + 7]);
        assert!(u >= 0.0 && u <= 1.0, "cylinder u={} out of range", u);
        assert!(v >= 0.0 && v <= 1.0, "cylinder v={} out of range", v);
    }
}

// --- Torus mesh ---

#[test]
fn torus_counts_and_bounds() {
    let (major, minor) = (12u32, 8u32);
    let (vertices, indices) = generate_torus(major, minor);
    let vertex_count = vertices.len() / VERTEX_STRIDE;

    // Watertight: exactly major*minor unique vertices (no seam duplicates).
    assert_eq!(vertex_count, (major * minor) as usize);
    // major*minor quads * 2 tris.
    assert_eq!(indices.len(), (major * minor * 2) as usize * 3);

    for &idx in &indices {
        assert!((idx as usize) < vertex_count, "torus index {} OOB", idx);
    }
}

#[test]
fn torus_every_vertex_used() {
    // A watertight torus must reference every generated vertex (no orphans).
    let (major, minor) = (10u32, 6u32);
    let (vertices, indices) = generate_torus(major, minor);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    let mut used = vec![false; vertex_count];
    for &idx in &indices {
        used[idx as usize] = true;
    }
    assert!(used.iter().all(|&u| u), "torus has unused vertices");
}

#[test]
fn torus_normals_unit_positions_bounded_uvs_in_range() {
    let (vertices, _) = generate_torus(16, 10);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    // R=0.7, r=0.3 → distance from Y axis in [0.4, 1.0], |y| <= 0.3.
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let (nx, ny, nz) = (vertices[base + 3], vertices[base + 4], vertices[base + 5]);
        assert!(((nx * nx + ny * ny + nz * nz).sqrt() - 1.0).abs() < 1e-4, "torus normal not unit");

        let (x, y, z) = (vertices[base], vertices[base + 1], vertices[base + 2]);
        let radial = (x * x + z * z).sqrt();
        assert!(radial >= 0.4 - 1e-4 && radial <= 1.0 + 1e-4, "torus radial {} out of range", radial);
        assert!(y.abs() <= 0.3 + 1e-4, "torus y {} out of range", y);

        let (u, v) = (vertices[base + 6], vertices[base + 7]);
        assert!(u >= 0.0 && u < 1.0, "torus u={} out of range", u);
        assert!(v >= 0.0 && v < 1.0, "torus v={} out of range", v);
    }
}

// --- Rounded cube mesh ---

#[test]
fn rounded_cube_counts_and_bounds() {
    // Same vertex/index topology as the flat cube (6 separate face grids).
    let (vertices, indices) = generate_rounded_cube(3);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    assert_eq!(vertex_count, 6 * (3 + 1) * (3 + 1));
    assert_eq!(indices.len(), 6 * 3 * 3 * 2 * 3);
    for &idx in &indices {
        assert!((idx as usize) < vertex_count, "rounded cube index {} OOB", idx);
    }
}

#[test]
fn rounded_cube_normals_unit_and_within_extents() {
    let (vertices, _) = generate_rounded_cube(4);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let (nx, ny, nz) = (vertices[base + 3], vertices[base + 4], vertices[base + 5]);
        assert!(((nx * nx + ny * ny + nz * nz).sqrt() - 1.0).abs() < 1e-5, "rounded cube normal not unit");
        // core clamp ±0.85 plus r=0.15 along a unit normal keeps coords in ±1.
        for k in 0..3 {
            let c = vertices[base + k];
            assert!(c >= -1.0 - 1e-5 && c <= 1.0 + 1e-5, "rounded cube coord {} out of extents", c);
        }
    }
}

#[test]
fn rounded_cube_uvs_in_range() {
    let (vertices, _) = generate_rounded_cube(4);
    let vertex_count = vertices.len() / VERTEX_STRIDE;
    for i in 0..vertex_count {
        let base = i * VERTEX_STRIDE;
        let (u, v) = (vertices[base + 6], vertices[base + 7]);
        assert!(u >= 0.0 && u <= 1.0, "rounded cube u={} out of range", u);
        assert!(v >= 0.0 && v <= 1.0, "rounded cube v={} out of range", v);
    }
}

#[test]
fn mesh_kind_all_and_labels() {
    // Phase 4 added three meshes; ALL must list all six with distinct labels.
    assert_eq!(MeshKind::ALL.len(), 6);
    let labels: Vec<&str> = MeshKind::ALL.iter().map(|k| k.label()).collect();
    for expected in ["Plane", "Sphere", "Cube", "Rounded Cube", "Cylinder", "Torus"] {
        assert!(labels.contains(&expected), "MeshKind labels missing `{}`", expected);
    }
}

// --- Directional shadow light-space fitting ---

/// Transform all 8 corners of the scene AABB (x,z ∈ [-1,1], y ∈ [-1.5,1.5] to
/// cover the ~0.5 height-displacement headroom) through `proj * view`,
/// perspective-divide, and assert every NDC component is finite and within the
/// clip cube [-1,1] (up to `NDC_TOL`). This proves the ortho box essentially
/// contains the scene for the given light direction — nothing meaningful gets
/// clipped out of the shadow map.
///
/// `NDC_TOL` is intentionally loose (0.05, not an epsilon): the AABB's extreme
/// corners sit at radius √(1+1.5²+1) ≈ 2.062, marginally OUTSIDE the documented
/// R = 2.0 bounding sphere the ortho box is sized to. So a light aimed such that
/// one of those corners lands nearly perpendicular to its view axis pushes it to
/// NDC ≈ 1.03. Those corners correspond only to heavily height-displaced cube
/// corners that may clip slightly in the shadow map — a known, accepted
/// limitation (same spirit as the non-watertight cube seam caveat), not a fit
/// bug. The tolerance keeps the test meaningful (catches gross mis-fits and
/// non-finite output) without over-fitting to that 3% corner overshoot.
fn assert_scene_fits(light_dir: glam::Vec3) {
    const NDC_TOL: f32 = 0.05;
    let (view, proj) = light_space_matrices(light_dir);
    let vp = proj * view;
    for &x in &[-1.0f32, 1.0] {
        for &y in &[-1.5f32, 1.5] {
            for &z in &[-1.0f32, 1.0] {
                let clip = vp * glam::Vec4::new(x, y, z, 1.0);
                assert!(clip.w.abs() > 1e-6, "degenerate w for corner ({x},{y},{z})");
                let ndc = clip.truncate() / clip.w;
                for (name, c) in [("x", ndc.x), ("y", ndc.y), ("z", ndc.z)] {
                    assert!(c.is_finite(), "NDC {name} not finite for dir {light_dir:?}");
                    assert!(
                        c >= -1.0 - NDC_TOL && c <= 1.0 + NDC_TOL,
                        "NDC {name}={c} out of [-1,1] for corner ({x},{y},{z}), dir {light_dir:?}",
                    );
                }
            }
        }
    }
}

#[test]
fn light_space_fits_default_direction() {
    // The viewer's default light dir (azimuth/elevation defaults resolve to
    // roughly (0.8, 1.0, 0.6).normalize()).
    assert_scene_fits(glam::Vec3::new(0.8, 1.0, 0.6).normalize());
}

#[test]
fn light_space_fits_grazing_direction() {
    // A near-horizontal (grazing) light: small +y component.
    assert_scene_fits(glam::Vec3::new(1.0, 0.05, 0.2).normalize());
}

#[test]
fn light_space_fits_straight_up() {
    // Straight up +Y exercises the look_at up-vector degeneracy guard.
    let dir = glam::Vec3::Y;
    assert_scene_fits(dir);

    // The resulting matrices must be finite and the combined transform
    // invertible (non-zero determinant) — i.e. the degeneracy guard produced a
    // valid, non-collapsed basis.
    let (view, proj) = light_space_matrices(dir);
    let vp = proj * view;
    for c in vp.to_cols_array() {
        assert!(c.is_finite(), "light-space matrix has non-finite entry");
    }
    assert!(
        vp.determinant().abs() > 1e-6,
        "light-space matrix not invertible (det ~ 0)"
    );
}

#[test]
fn cast_slice_to_bytes_roundtrip() {
    let data: Vec<f32> = vec![1.0, 2.0, 3.0];
    let bytes = cast_slice_to_bytes(&data);
    assert_eq!(bytes.len(), 3 * std::mem::size_of::<f32>());
    let ptr = bytes.as_ptr() as *const f32;
    unsafe {
        assert_eq!(*ptr, 1.0);
        assert_eq!(*ptr.add(1), 2.0);
        assert_eq!(*ptr.add(2), 3.0);
    }
}
