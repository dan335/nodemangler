- [ ] Frame / Comment Nodes
- [ ] Dot / Reroute Nodes

## Simulation nodes (new `images/simulation/` category)

Generators that simulate the physical process behind a material's look
(the caustics-node approach) instead of layering random noises. Convention:
guidance-map inputs are optional — unconnected, the node falls back to an
internal seed-derived map, so every node also works standalone.

Priority batch:
- [ ] Drying cracks — crack tips nucleate at weak points (optional weakness map), propagate perpendicular to shrinkage stress, relieve stress around themselves, T-junction into older cracks; hierarchical plates. Covers mud, glaze crackle, dried paint; anisotropic stress variant covers bark.
- [ ] Hydraulic erosion — droplet sim over a heightmap (optional height input): drops roll downhill, pick up sediment at speed, deposit when slowing; carves gullies, ridges, sediment fans. Complements the existing thermal-only erosion node.
- [ ] Frost / dendrites — diffusion-limited aggregation: particles random-walk and stick to the growing cluster; seed from points (frost stars) or an edge (window-frame frost). Also mineral dendrites, coral.
- [ ] Sand ripples / dunes — Werner slab model: wind hops sand slabs downwind with shadow-zone deposition; asymmetric ripples with Y-junction defects. Also snow drifts.
- [ ] Concrete — casting sim: size-graded aggregate sphere packing (soft bumps), rising-bubble bugholes (power-law pits with crisp rims), hairline shrinkage cracks, faint trowel arcs; composited as height.

Later:
- [ ] Rain ripples / water surface — damped circular wave packets from drop impacts with real interference; directional wave-spectrum variant for wind-blown water. Pairs with caustics.
- [ ] Rust (percolation) — pits nucleate (optional moisture map), corrosion fronts grow with downward bias, staining halos and bleed streaks below pits.
- [ ] Scorch / burn — fire-front propagation over a fuel map (optional) from ignition points; time-since-burn maps to clean→scorched→charred with fractal boundary.
- [ ] Zinc spangle — crystallization race: seeds nucleate at random times, grow radially with six dendritic arms until fronts collide; per-grain facet brightness.
- [ ] Tide marks / efflorescence — capillary percolation with mineral deposition at each drying front over wet/dry cycles; nested contour rings.
- [ ] Paint drips (viscous) — fluid blobs flow downward, thin into trails, bead at the stop point; distinct from leaks (stain streaks).
- [ ] Lightning upgrade — dielectric breakdown model (Laplacian growth) for physically-correct branching statistics.
