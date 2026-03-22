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
