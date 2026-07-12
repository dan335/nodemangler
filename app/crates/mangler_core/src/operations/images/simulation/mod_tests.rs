//! Tests for the shared simulation helpers, mainly the labeled distance
//! transform (validated against a brute-force reference).

use super::distance_field_labeled;

/// Brute-force nearest-site search: for every pixel, scan all sites and keep
/// the smallest squared distance. Ground truth for the separable transform.
fn brute_force(sites: &[bool], w: usize, h: usize) -> Vec<f64> {
    let site_coords: Vec<(usize, usize)> = (0..h)
        .flat_map(|y| (0..w).filter(move |&x| sites[y * w + x]).map(move |x| (x, y)))
        .collect();
    (0..h)
        .flat_map(|y| {
            let site_coords = &site_coords;
            (0..w).map(move |x| {
                site_coords
                    .iter()
                    .map(|&(sx, sy)| {
                        let dx = x as f64 - sx as f64;
                        let dy = y as f64 - sy as f64;
                        dx * dx + dy * dy
                    })
                    .fold(1e20_f64, f64::min)
            })
        })
        .collect()
}

#[test]
fn labeled_dt_matches_brute_force() {
    let w = 23;
    let h = 17;
    // Deterministic scattered sites.
    let mut rng = fastrand::Rng::with_seed(7);
    let sites: Vec<bool> = (0..w * h).map(|_| rng.f64() < 0.05).collect();
    assert!(sites.iter().any(|&s| s), "seed produced no sites");

    let (d2, labels) = distance_field_labeled(&sites, w, h);
    let expected = brute_force(&sites, w, h);

    for i in 0..w * h {
        assert!(
            (d2[i] - expected[i]).abs() < 1e-9,
            "distance mismatch at {}: got {}, expected {}",
            i, d2[i], expected[i]
        );
        // The label must point at a real site whose distance equals the
        // reported minimum (the nearest site may be non-unique).
        let l = labels[i] as usize;
        assert!(sites[l], "label at {} points at a non-site cell {}", i, l);
        let lx = (l % w) as f64;
        let ly = (l / w) as f64;
        let x = (i % w) as f64;
        let y = (i / w) as f64;
        let ld2 = (x - lx) * (x - lx) + (y - ly) * (y - ly);
        assert!(
            (ld2 - expected[i]).abs() < 1e-9,
            "label at {} is not a nearest site: label dist {}, expected {}",
            i, ld2, expected[i]
        );
    }
}

#[test]
fn labeled_dt_single_site() {
    let w = 9;
    let h = 5;
    let mut sites = vec![false; w * h];
    let site = 2 * w + 6;
    sites[site] = true;
    let (d2, labels) = distance_field_labeled(&sites, w, h);
    assert_eq!(d2[site], 0.0);
    for i in 0..w * h {
        assert_eq!(labels[i], site as u32);
    }
}

#[test]
fn labeled_dt_no_sites() {
    let w = 6;
    let h = 4;
    let sites = vec![false; w * h];
    let (d2, labels) = distance_field_labeled(&sites, w, h);
    for i in 0..w * h {
        assert!(d2[i] >= 1e20);
        assert_eq!(labels[i], u32::MAX);
    }
}
