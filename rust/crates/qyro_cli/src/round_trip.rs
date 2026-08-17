//! The test that decides whether the optical channel is real.
//!
//! Everything else about this channel can be asserted from the inside: that the
//! quiet zone is four modules, that two module rows share a line, that the
//! fountain rebuilds a payload from frames chosen at random. All of it can be
//! true of a drawing **no decoder will ever accept**, and this repository has
//! shipped that shape of defect five times — written, tested, and unreachable.
//!
//! So: draw exactly what `qyro beam` draws, rasterise it exactly as a camera
//! would see it, hand it to a real QR decoder that has never seen this project,
//! and feed what comes out to the fountain until the original bytes come back.
//!
//! # No fixture files, deliberately
//!
//! The obvious shape is a directory of PNGs. Fixtures rot: a renderer change
//! makes them stale, and a stale fixture fails as «the renderer broke» while
//! the truth is that the picture is old. Rasterising in memory has no such
//! failure mode, and it needs no image decoder in the graph.
//!
//! # What this does NOT prove
//!
//! A camera. Lens blur, rolling shutter, moiré, glare, a screen at an angle,
//! and the phone's own decoder are all outside this, and they are where an
//! optical channel actually fails. That is phase 19, with hardware, and the
//! blank stays blank until then.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

/// Turns the drawn text back into the pixels a camera would see.
///
/// Each half-block character is two module rows, so one character becomes a
/// `SCALE` × `2 * SCALE` patch. **Dark modules become dark pixels**, undoing the
/// deliberate inversion in the renderer — a camera pointed at a light-on-dark
/// terminal sees exactly that.
fn rasterise(drawing: &str) -> (Vec<u8>, usize, usize) {
    const SCALE: usize = 4;

    let rows: Vec<&str> = drawing.lines().collect();
    let columns = rows
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0);
    let width = columns * SCALE;
    let height = rows.len() * SCALE * 2;
    // 255 is white; a QR needs a light field, so that is the background.
    let mut pixels = vec![255_u8; width * height];

    for (row_index, row) in rows.iter().enumerate() {
        for (column_index, cell) in row.chars().enumerate() {
            // The renderer draws *light* as ink, so read the halves back that
            // way: a filled block means both module rows are light.
            let (top_light, bottom_light) = match cell {
                '\u{2588}' => (true, true),
                '\u{2580}' => (true, false),
                '\u{2584}' => (false, true),
                _ => (false, false),
            };
            for (half, light) in [top_light, bottom_light].into_iter().enumerate() {
                if light {
                    continue;
                }
                let y0 = row_index * SCALE * 2 + half * SCALE;
                let x0 = column_index * SCALE;
                for y in y0..y0 + SCALE {
                    for x in x0..x0 + SCALE {
                        if let Some(pixel) = pixels.get_mut(y * width + x) {
                            *pixel = 0;
                        }
                    }
                }
            }
        }
    }
    (pixels, width, height)
}

/// Reads the bytes back out of a rasterised drawing with a real QR decoder.
fn scan(drawing: &str) -> Option<Vec<u8>> {
    let (pixels, width, height) = rasterise(drawing);
    // `prepare_from_greyscale` rather than a decoded image file: no image crate
    // in the graph, and nothing on disk to go stale.
    let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| {
        pixels.get(y * width + x).copied().unwrap_or(255)
    });
    let grids = prepared.detect_grids();
    let grid = grids.first()?;
    let mut bytes = Vec::new();
    grid.decode_to(&mut bytes).ok()?;
    Some(bytes)
}

#[test]
fn a_real_decoder_reads_what_the_terminal_draws() {
    // The single assertion this file exists for. If the inversion were
    // backwards, or the quiet zone missing, or the half-blocks mismatched, every
    // other test in this crate would still pass and this one would not.
    let payload = b"QYRO1|192.168.1.9:49517|ab12cd34ab12cd34ab12cd34ab12cd34";
    let drawing = crate::optical::draw(payload).expect("a code this small always fits");

    let read_back = scan(&drawing).expect(
        "a real QR decoder could not find a code in what the terminal drew -- \
         which is the failure every other test in this crate is blind to",
    );
    assert_eq!(read_back, payload.to_vec());
}

#[test]
fn a_file_survives_the_whole_optical_channel_end_to_end() {
    // Draw, rasterise, decode, feed the fountain, rebuild. Every stage of
    // `qyro beam` except the camera, and **frames are dropped on the way** —
    // because a channel that only works when nothing is missed is the channel
    // ADR-0044 §4 exists to avoid.
    let original: Vec<u8> = (0..900).map(|i| ((i * 37 + 11) % 251) as u8).collect();
    let block_size: u16 = 180;
    let shape = qyro_fountain::Shape {
        payload_len: u32::try_from(original.len()).expect("small"),
        block_size,
    };
    let blocks = qyro_fountain::split(&original, block_size);

    let mut decoder = qyro_fountain::Decoder::new(shape);
    let mut seed = 1_u64;
    let mut scanned = 0;
    while !decoder.is_complete() && seed < 400 {
        // Every fourth frame never reaches the camera.
        if seed % 4 != 0 {
            let frame = qyro_fountain::encode(&blocks, shape, seed);
            let wire = qyro_fountain::encode_frame(&frame);
            let drawing = crate::optical::draw(&wire).expect("a frame fits a QR");
            let read_back = scan(&drawing).expect("the decoder lost a frame it should have read");
            let recovered =
                qyro_fountain::decode_frame(&read_back).expect("the bytes survived the round trip");
            decoder.accept(&recovered);
            scanned += 1;
        }
        seed += 1;
    }

    assert!(
        decoder.is_complete(),
        "the file did not come back after {scanned} scanned frames"
    );
    assert_eq!(
        decoder.finish().as_deref(),
        Some(original.as_slice()),
        "the file came back different, which is worse than not coming back"
    );
}

#[test]
fn and_the_scanner_is_not_simply_agreeing_with_everything() {
    // The control. A `scan` that returned the payload it was handed, or that
    // found a code in anything, would pass both tests above and prove nothing.
    let noise = "\u{2588}\u{2584}\u{2580} \u{2588}\u{2584}\u{2580} \n\u{2580} \u{2588}\u{2584}\n";
    assert!(
        scan(noise).is_none(),
        "the scanner found a QR code in three lines of noise"
    );
}
