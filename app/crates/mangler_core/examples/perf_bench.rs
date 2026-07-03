//! Performance benchmark harness for mangler_core image operations.
//!
//! Runs each operation on a deterministic test image and prints median
//! wall-clock time over several runs. Used to record before/after numbers
//! for optimization work. Run with:
//!
//! ```sh
//! cargo run --release --example perf_bench
//! ```

use mangler_core::color::blend::BlendMode;
use mangler_core::color::color_spaces::ColorSpace;
use mangler_core::float_image::FloatImage;
use mangler_core::get_id;
use mangler_core::input::Input;
use mangler_core::value::Value;
use std::sync::Arc;
use std::time::Instant;

const RUNS: usize = 3;

/// Deterministic structured test image: gradients, rings, and hash noise.
fn test_image(w: u32, h: u32, ch: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, ch);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let fy = y as f32 / h as f32;
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let ring = (d * 0.15).sin() * 0.5 + 0.5;
            // cheap deterministic hash noise
            let mut n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
            n = (n ^ (n >> 13)).wrapping_mul(1274126177);
            let noise = (n & 0xffff) as f32 / 65535.0;
            let px = [
                (fx * 0.7 + ring * 0.2 + noise * 0.1).clamp(0.0, 1.0),
                (fy * 0.7 + ring * 0.3).clamp(0.0, 1.0),
                (ring * 0.6 + noise * 0.4).clamp(0.0, 1.0),
                1.0,
            ];
            img.put_pixel(x, y, &px[..ch as usize]);
        }
    }
    Arc::new(img)
}

fn set(inputs: &mut [Input], name: &str, value: Value) {
    match inputs.iter_mut().find(|i| i.name == name) {
        Some(input) => input.value = value,
        None => panic!("no input named '{}'", name),
    }
}

fn img_value(img: &Arc<FloatImage>) -> Value {
    Value::Image { data: img.clone(), change_id: get_id() }
}

macro_rules! bench {
    ($name:expr, $ty:ty, $build:expr) => {{
        let build = $build;
        // warmup
        {
            let mut inputs: Vec<Input> = build();
            let r = <$ty>::run(&mut inputs).await;
            if let Err(e) = r {
                println!("{:32} FAILED: {:?}", $name, e);
            }
        }
        let mut times: Vec<f64> = Vec::new();
        for _ in 0..RUNS {
            let mut inputs: Vec<Input> = build();
            let t = Instant::now();
            let _ = <$ty>::run(&mut inputs).await;
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        println!("{:32} {:9.1} ms", $name, times[times.len() / 2]);
    }};
}

#[tokio::main]
async fn main() {
    use mangler_core::operations::colors::sample_image::most_common_colors::OpColorSampleMostCommonColors;
    use mangler_core::operations::images::adjustments::gradient_dynamic::OpImageAdjustmentGradientDynamic;
    use mangler_core::operations::images::adjustments::gradient_map::OpImageAdjustmentGradientMap;
    use mangler_core::operations::images::adjustments::hsl::OpImageAdjustmentHsl;
    use mangler_core::operations::images::adjustments::levels::OpImageAdjustmentLevels;
    use mangler_core::operations::images::blur::blur::OpImageAdjustmentBlur;
    use mangler_core::operations::images::blur::directional_blur::OpImageAdjustmentDirectionalBlur;
    use mangler_core::operations::images::blur::non_uniform_blur::OpImageAdjustmentNonUniformBlur;
    use mangler_core::operations::images::blur::radial_blur::OpImageAdjustmentRadialBlur;
    use mangler_core::operations::images::blur::slope_blur::OpImageAdjustmentSlopeBlur;
    use mangler_core::operations::images::channels::shuffle::OpImageChannelShuffle;
    use mangler_core::operations::images::channels::split::OpImageChannelSplit;
    use mangler_core::operations::images::combine::blend::OpImageCombineBlend;
    use mangler_core::operations::images::filter::anisotropic_diffusion::OpImageAdjustmentAnisotropicDiffusion;
    use mangler_core::operations::images::filter::anisotropic_kuwahara::OpImageAdjustmentAnisotropicKuwahara;
    use mangler_core::operations::images::filter::bilateral::OpImageAdjustmentBilateral;
    use mangler_core::operations::images::filter::canny::OpImageAdjustmentCanny;
    use mangler_core::operations::images::filter::dog::OpImageAdjustmentDog;
    use mangler_core::operations::images::filter::erode::OpImageAdjustmentErode;
    use mangler_core::operations::images::filter::guided::OpImageAdjustmentGuided;
    use mangler_core::operations::images::filter::kuwahara::OpImageAdjustmentKuwahara;
    use mangler_core::operations::images::filter::median::OpImageAdjustmentMedian;
    use mangler_core::operations::images::filter::non_local_means::OpImageAdjustmentNonLocalMeans;
    use mangler_core::operations::images::filter::oil_paint::OpImageAdjustmentOilPaint;
    use mangler_core::operations::images::filter::open::OpImageAdjustmentOpen;
    use mangler_core::operations::images::filter::snn::OpImageAdjustmentSnn;
    use mangler_core::operations::images::filter::toon::OpImageAdjustmentToon;
    use mangler_core::operations::images::filter::vector_morphology::OpImageAdjustmentVectorMorphology;
    use mangler_core::operations::images::fx::drop_shadow::OpImageFxDropShadow;
    use mangler_core::operations::images::fx::outer_glow::OpImageFxOuterGlow;
    use mangler_core::operations::images::noise::blue_noise::OpImageNoiseBlue;
    use mangler_core::operations::images::noise::curl::OpImageNoiseCurl;
    use mangler_core::operations::images::noise::erosion::OpImageNoiseErosion;
    use mangler_core::operations::images::noise::gabor::OpImageNoiseGabor;
    use mangler_core::operations::images::pbr::ao_from_height::OpImagePbrAoFromHeight;
    use mangler_core::operations::images::pbr::bevel::OpImagePbrBevel;
    use mangler_core::operations::images::pbr::curvature::OpImagePbrCurvature;
    use mangler_core::operations::images::transform::kaleidoscope::OpImageTransformKaleidoscope;
    use mangler_core::operations::images::transform::polar_coordinates::OpImageTransformPolarCoordinates;
    use mangler_core::operations::images::transform::rotate_around_center::OpImageTransformRotateAroundCenter;
    use mangler_core::operations::images::transform::spherize::OpImageTransformSpherize;
    use mangler_core::operations::images::transform::swirl::OpImageTransformSwirl;
    use mangler_core::operations::images::transform::warp::OpImageTransformWarp;

    println!("perf_bench: 512x512 RGBA unless noted, median of {} runs", RUNS);

    let img = test_image(512, 512, 4);
    let img256 = test_image(256, 256, 4);
    let img2048 = test_image(2048, 2048, 4);

    // --- blur / filter ---
    bench!("blur sigma=8", OpImageAdjustmentBlur, || {
        let mut i = OpImageAdjustmentBlur::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "sigma", Value::Decimal(8.0));
        i
    });
    bench!("non_local_means 256px s=5 p=3", OpImageAdjustmentNonLocalMeans, || {
        let mut i = OpImageAdjustmentNonLocalMeans::create_inputs();
        set(&mut i, "image", img_value(&img256));
        set(&mut i, "search radius", Value::Integer(5));
        set(&mut i, "patch radius", Value::Integer(3));
        i
    });
    bench!("bilateral r=8", OpImageAdjustmentBilateral, || {
        let mut i = OpImageAdjustmentBilateral::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "radius", Value::Integer(8));
        i
    });
    bench!("guided r=8", OpImageAdjustmentGuided, || {
        let mut i = OpImageAdjustmentGuided::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "radius", Value::Integer(8));
        i
    });
    bench!("median r=6", OpImageAdjustmentMedian, || {
        let mut i = OpImageAdjustmentMedian::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "radius", Value::Integer(6));
        i
    });
    bench!("kuwahara r=8", OpImageAdjustmentKuwahara, || {
        let mut i = OpImageAdjustmentKuwahara::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "radius", Value::Integer(8));
        i
    });
    bench!("anisotropic_kuwahara r=8", OpImageAdjustmentAnisotropicKuwahara, || {
        let mut i = OpImageAdjustmentAnisotropicKuwahara::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "radius", Value::Integer(8));
        i
    });
    bench!("snn r=6", OpImageAdjustmentSnn, || {
        let mut i = OpImageAdjustmentSnn::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "radius", Value::Integer(6));
        i
    });
    bench!("oil_paint r=6", OpImageAdjustmentOilPaint, || {
        let mut i = OpImageAdjustmentOilPaint::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "radius", Value::Integer(6));
        i
    });
    bench!("dog sigma=4", OpImageAdjustmentDog, || {
        let mut i = OpImageAdjustmentDog::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "sigma", Value::Decimal(4.0));
        i
    });
    bench!("canny sigma=3", OpImageAdjustmentCanny, || {
        let mut i = OpImageAdjustmentCanny::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "sigma", Value::Decimal(3.0));
        i
    });
    bench!("toon defaults", OpImageAdjustmentToon, || {
        let mut i = OpImageAdjustmentToon::create_inputs();
        set(&mut i, "image", img_value(&img));
        i
    });
    bench!("anisotropic_diffusion it=30", OpImageAdjustmentAnisotropicDiffusion, || {
        let mut i = OpImageAdjustmentAnisotropicDiffusion::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "iterations", Value::Integer(30));
        i
    });
    bench!("vector_morphology r=8", OpImageAdjustmentVectorMorphology, || {
        let mut i = OpImageAdjustmentVectorMorphology::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "radius", Value::Integer(8));
        i
    });
    bench!("erode r=12", OpImageAdjustmentErode, || {
        let mut i = OpImageAdjustmentErode::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "radius", Value::Integer(12));
        i
    });
    bench!("open r=8", OpImageAdjustmentOpen, || {
        let mut i = OpImageAdjustmentOpen::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "radius", Value::Integer(8));
        i
    });
    bench!("directional_blur s=32", OpImageAdjustmentDirectionalBlur, || {
        let mut i = OpImageAdjustmentDirectionalBlur::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "samples", Value::Integer(32));
        i
    });
    bench!("radial_blur s=32", OpImageAdjustmentRadialBlur, || {
        let mut i = OpImageAdjustmentRadialBlur::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "samples", Value::Integer(32));
        i
    });
    bench!("slope_blur s=32", OpImageAdjustmentSlopeBlur, || {
        let mut i = OpImageAdjustmentSlopeBlur::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "slope map", img_value(&img));
        set(&mut i, "samples", Value::Integer(32));
        i
    });
    bench!("non_uniform_blur s=32", OpImageAdjustmentNonUniformBlur, || {
        let mut i = OpImageAdjustmentNonUniformBlur::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "blur map", img_value(&img));
        set(&mut i, "samples", Value::Integer(32));
        i
    });

    // --- transform ---
    bench!("swirl", OpImageTransformSwirl, || {
        let mut i = OpImageTransformSwirl::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "angle", Value::Decimal(120.0));
        i
    });
    bench!("warp", OpImageTransformWarp, || {
        let mut i = OpImageTransformWarp::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "displacement", img_value(&img));
        i
    });
    bench!("kaleidoscope", OpImageTransformKaleidoscope, || {
        let mut i = OpImageTransformKaleidoscope::create_inputs();
        set(&mut i, "image", img_value(&img));
        i
    });
    bench!("polar_coordinates", OpImageTransformPolarCoordinates, || {
        let mut i = OpImageTransformPolarCoordinates::create_inputs();
        set(&mut i, "image", img_value(&img));
        i
    });
    bench!("spherize", OpImageTransformSpherize, || {
        let mut i = OpImageTransformSpherize::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "amount", Value::Decimal(0.8));
        i
    });
    bench!("rotate_around_center 37deg", OpImageTransformRotateAroundCenter, || {
        let mut i = OpImageTransformRotateAroundCenter::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "degrees", Value::Decimal(37.0));
        i
    });

    // --- adjustments ---
    bench!("levels mid=0.3", OpImageAdjustmentLevels, || {
        let mut i = OpImageAdjustmentLevels::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "in mid", Value::Decimal(0.3));
        i
    });
    bench!("hsl hue+60", OpImageAdjustmentHsl, || {
        let mut i = OpImageAdjustmentHsl::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "hue", Value::Decimal(60.0));
        i
    });
    bench!("gradient_map", OpImageAdjustmentGradientMap, || {
        let mut i = OpImageAdjustmentGradientMap::create_inputs();
        set(&mut i, "image", img_value(&img));
        i
    });
    bench!("gradient_dynamic", OpImageAdjustmentGradientDynamic, || {
        let mut i = OpImageAdjustmentGradientDynamic::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "gradient", img_value(&img));
        set(&mut i, "vector field", img_value(&img));
        i
    });

    // --- combine / channels ---
    bench!("blend srgb over", OpImageCombineBlend, || {
        let mut i = OpImageCombineBlend::create_inputs();
        set(&mut i, "background", img_value(&img));
        set(&mut i, "foreground", img_value(&img));
        i
    });
    bench!("blend lab multiply", OpImageCombineBlend, || {
        let mut i = OpImageCombineBlend::create_inputs();
        set(&mut i, "background", img_value(&img));
        set(&mut i, "foreground", img_value(&img));
        set(&mut i, "blend mode", Value::BlendMode(BlendMode::Multiply));
        set(&mut i, "color space", Value::ColorSpace(ColorSpace::Lab));
        i
    });
    bench!("channel_split", OpImageChannelSplit, || {
        let mut i = OpImageChannelSplit::create_inputs();
        set(&mut i, "image", img_value(&img));
        i
    });
    bench!("channel_shuffle", OpImageChannelShuffle, || {
        let mut i = OpImageChannelShuffle::create_inputs();
        set(&mut i, "image", img_value(&img));
        i
    });

    // --- fx ---
    bench!("drop_shadow blur=12", OpImageFxDropShadow, || {
        let mut i = OpImageFxDropShadow::create_inputs();
        set(&mut i, "mask", img_value(&img));
        set(&mut i, "blur radius", Value::Decimal(12.0));
        i
    });
    bench!("outer_glow r=12", OpImageFxOuterGlow, || {
        let mut i = OpImageFxOuterGlow::create_inputs();
        set(&mut i, "mask", img_value(&img));
        set(&mut i, "radius", Value::Decimal(12.0));
        i
    });

    // --- pbr ---
    bench!("bevel dist=32", OpImagePbrBevel, || {
        let mut i = OpImagePbrBevel::create_inputs();
        set(&mut i, "mask", img_value(&img));
        set(&mut i, "distance", Value::Integer(32));
        i
    });
    bench!("ao_from_height s=16", OpImagePbrAoFromHeight, || {
        let mut i = OpImagePbrAoFromHeight::create_inputs();
        set(&mut i, "image", img_value(&img));
        set(&mut i, "samples", Value::Integer(16));
        i
    });
    bench!("curvature", OpImagePbrCurvature, || {
        let mut i = OpImagePbrCurvature::create_inputs();
        set(&mut i, "image", img_value(&img));
        i
    });

    // --- noise ---
    bench!("gabor 512", OpImageNoiseGabor, || {
        let mut i = OpImageNoiseGabor::create_inputs();
        set(&mut i, "width", Value::Integer(512));
        set(&mut i, "height", Value::Integer(512));
        i
    });
    bench!("erosion 256 it=100", OpImageNoiseErosion, || {
        let mut i = OpImageNoiseErosion::create_inputs();
        set(&mut i, "width", Value::Integer(256));
        set(&mut i, "height", Value::Integer(256));
        set(&mut i, "iterations", Value::Integer(100));
        i
    });
    bench!("blue_noise 512 r=32", OpImageNoiseBlue, || {
        let mut i = OpImageNoiseBlue::create_inputs();
        set(&mut i, "width", Value::Integer(512));
        set(&mut i, "height", Value::Integer(512));
        set(&mut i, "radius", Value::Integer(32));
        i
    });
    bench!("curl 512", OpImageNoiseCurl, || {
        let mut i = OpImageNoiseCurl::create_inputs();
        set(&mut i, "width", Value::Integer(512));
        set(&mut i, "height", Value::Integer(512));
        i
    });

    // --- colors ---
    bench!("most_common_colors", OpColorSampleMostCommonColors, || {
        let mut i = OpColorSampleMostCommonColors::create_inputs();
        set(&mut i, "image", img_value(&img));
        i
    });

    // --- FloatImage primitives (thumbnail path) ---
    {
        let mut times: Vec<f64> = Vec::new();
        let _ = img2048.resize_fit(150, 150); // warmup
        for _ in 0..RUNS {
            let t = Instant::now();
            let r = img2048.resize_fit(150, 150);
            std::hint::black_box(&r);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        println!("{:32} {:9.1} ms", "resize_fit 2048->150", times[times.len() / 2]);
    }
    {
        let mut times: Vec<f64> = Vec::new();
        let _ = img2048.to_rgba8(); // warmup
        for _ in 0..RUNS {
            let t = Instant::now();
            let r = img2048.to_rgba8();
            std::hint::black_box(&r);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        println!("{:32} {:9.1} ms", "to_rgba8 2048", times[times.len() / 2]);
    }
}
