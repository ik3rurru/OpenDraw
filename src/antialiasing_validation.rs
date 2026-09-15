//! Reproducible visual/performance checks; run explicitly in release mode:
//! cargo test --release antialiasing_validation -- --ignored --nocapture --test-threads=1

use std::{hint::black_box, path::Path, time::Instant};

use crate::{
    document::{CanvasView, Document},
    file::{self, ImageFormat},
    graphics::{Color, FrameBuffer, Rect},
    tools::{BrushSample, BrushTool, EraserTool, Tool},
};

fn sample(x: f32, y: f32, pressure: f32) -> BrushSample {
    BrushSample {
        x,
        y,
        pressure,
        tilt_x: 0.0,
        tilt_y: 0.0,
        rotation: 0.0,
        timestamp: 0,
    }
}

fn quality_document(case: u32) -> Document {
    let background = match case {
        0 | 1 => Color::rgb(255, 255, 255),
        2 => Color::rgba(0, 0, 0, 0),
        _ => Color::rgb(31, 116, 184),
    };
    let mut document = Document::new(144, 80, background).unwrap();
    let pixels = &mut document.active_layer_mut().pixels;
    let mut brush = BrushTool::default();
    brush.settings.radius = match case {
        0 => 0,
        1 => 10,
        _ => 4,
    };
    brush.settings.color = if case == 2 {
        Color::rgb(211, 45, 83)
    } else {
        Color::rgb(20, 24, 31)
    };
    brush.settings.opacity = if case == 2 { 90 } else { 255 };
    let mut eraser = EraserTool::default();
    eraser.settings.radius = 7;
    eraser.settings.opacity = 190;
    for i in 0..=256 {
        let t = i as f32 / 256.0;
        let x = 10.25 + 124.0 * t;
        let y = if case == 0 {
            15.25 + 50.0 * t
        } else if case == 1 {
            38.75
        } else {
            40.25 + (t * std::f32::consts::TAU).sin() * 14.0
        };
        let pressure = if case == 1 || case == 3 {
            0.1 + 0.9 * t
        } else {
            1.0
        };
        let tool: &mut dyn Tool = if case == 3 { &mut eraser } else { &mut brush };
        if i == 0 {
            tool.pointer_down(pixels, sample(x, y, pressure));
        } else {
            tool.pointer_move(pixels, sample(x, y, pressure));
        }
    }
    brush.pointer_up(pixels);
    eraser.pointer_up(pixels);
    document
}

#[test]
#[ignore = "writes visual QA artifacts to target/antialiasing"]
fn export_visual_comparison() {
    let directory = Path::new("target/antialiasing");
    std::fs::create_dir_all(directory).unwrap();
    let mut frame = FrameBuffer::default();
    frame.resize(1104, 872).unwrap();
    frame.clear(Color::rgb(24, 28, 36));
    let heading = Color::rgb(243, 245, 249);
    let muted = Color::rgb(168, 185, 207);
    frame.draw_text(24, 20, "OPENDRAW / CIRCULAR ANTIALIASING", heading, 2);
    frame.draw_text(24, 54, "100% / FULL STROKE", muted, 1);
    frame.draw_text(288, 54, "800% / CROP / NEAREST PIXELS", muted, 1);
    for (case, label) in [
        "1 PX DIAGONAL",
        "PRESSURE 10-100%",
        "CURVE / ALPHA",
        "ERASER / ALPHA",
    ]
    .into_iter()
    .enumerate()
    {
        let document = quality_document(case as u32);
        let y = 88 + case as i32 * 192;
        frame.draw_text(24, y, label, heading, 1);
        CanvasView {
            zoom: 1.0,
            offset_x: 24.0,
            offset_y: (y + 24) as f32,
        }
        .render(&document, &mut frame, Rect::new(24, y + 24, 144, 80));
        CanvasView {
            zoom: 8.0,
            offset_x: 288.0 - 24.0 * 8.0,
            offset_y: (y + 24) as f32 - 30.0 * 8.0,
        }
        .render(&document, &mut frame, Rect::new(288, y + 24, 768, 160));
        let name = format!("stroke-{case}");
        file::export(
            &document,
            &directory.join(format!("{name}.png")),
            ImageFormat::Png,
        )
        .unwrap();
        file::save(&document, &directory.join(format!("{name}.odraw"))).unwrap();
    }
    let mut sheet = Document::new(frame.width, frame.height, Color::rgba(0, 0, 0, 0)).unwrap();
    sheet.active_layer_mut().pixels.pixels = frame.pixels;
    file::export(&sheet, &directory.join("quality.png"), ImageFormat::Png).unwrap();
    println!(
        "Visual comparison: {}",
        directory.join("quality.png").display()
    );
}

#[test]
#[ignore = "manual release-mode timing, without hardware-dependent pass thresholds"]
fn benchmark_large_dabs_and_fast_strokes() {
    let clear = Color::rgba(0, 0, 0, 0);
    let mut document = Document::new(2048, 512, clear).unwrap();
    let pixels = &mut document.active_layer_mut().pixels;
    println!("512 dabs; median of 5 runs, no allocation inside the timed loop");
    for radius in [0, 4, 32, 127] {
        for opacity in [90, 255] {
            for erase in [false, true] {
                let mut timings = [0.0_f64; 5];
                for timing in &mut timings {
                    pixels.pixels.fill(if erase {
                        Color::rgb(40, 80, 190).as_u32()
                    } else {
                        0
                    });
                    let start = Instant::now();
                    for i in 0..512 {
                        let x = 128.25 + (i * 31 % 1792) as f32;
                        let y = 200.75 + (i % 64) as f32;
                        if erase {
                            pixels.erase_circle(x, y, radius as f32 + 0.5, opacity);
                        } else {
                            pixels.stamp_circle(
                                x,
                                y,
                                radius as f32 + 0.5,
                                Color::rgba(211, 45, 83, opacity),
                            );
                        }
                    }
                    black_box(&pixels.pixels);
                    *timing = start.elapsed().as_secs_f64() * 1000.0;
                }
                timings.sort_by(f64::total_cmp);
                println!(
                    "{} diameter={} opacity={opacity}: {:.3} ms/512 dabs ({:.1} us/dab)",
                    if erase { "erase" } else { "paint" },
                    radius * 2 + 1,
                    timings[2],
                    timings[2] * 1000.0 / 512.0
                );
            }
        }
    }
    println!("Fast move: 1792 document pixels in one packet; median of 31 strokes");
    for radius in [0, 4, 32, 127] {
        for erase in [false, true] {
            let mut brush = BrushTool::default();
            brush.settings.radius = radius;
            brush.settings.opacity = 90;
            let mut eraser = EraserTool::default();
            eraser.settings.radius = radius;
            eraser.settings.opacity = 90;
            let tool: &mut dyn Tool = if erase { &mut eraser } else { &mut brush };
            let mut timings = [0.0_f64; 31];
            for timing in &mut timings {
                pixels.pixels.fill(if erase {
                    Color::rgb(40, 80, 190).as_u32()
                } else {
                    0
                });
                let start = Instant::now();
                tool.pointer_down(pixels, sample(128.25, 180.75, 0.1));
                tool.pointer_move(pixels, sample(1920.25, 330.75, 1.0));
                tool.pointer_up(pixels);
                black_box(&pixels.pixels);
                *timing = start.elapsed().as_secs_f64() * 1000.0;
            }
            timings.sort_by(f64::total_cmp);
            println!(
                "{} diameter={}: {:.3} ms/stroke",
                if erase { "erase" } else { "paint" },
                radius * 2 + 1,
                timings[15]
            );
        }
    }
}
