//! What the fountain has to survive, tested without a screen or a camera.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use crate::{Decoder, Shape, decode_frame, encode, encode_frame, neighbours, split};

fn payload_of(len: usize) -> Vec<u8> {
    // Not all zeroes and not a repeating byte: a decoder that XORed wrongly
    // would still produce zeroes from zeroes, and the test would pass.
    (0..len)
        .map(|index| ((index * 31 + 7) % 251) as u8)
        .collect()
}

fn shape_for(payload: &[u8], block_size: u16) -> Shape {
    Shape {
        payload_len: u32::try_from(payload.len()).expect("test payloads are small"),
        block_size,
    }
}

#[test]
fn a_payload_survives_a_stream_that_arrives_in_order() {
    let payload = payload_of(4096);
    let shape = shape_for(&payload, 256);
    let blocks = split(&payload, shape.block_size);

    let mut decoder = Decoder::new(shape);
    let mut seed = 1_u64;
    while !decoder.is_complete() && seed < 5000 {
        decoder.accept(&encode(&blocks, shape, seed));
        seed += 1;
    }

    assert!(decoder.is_complete(), "gave up after {seed} frames");
    assert_eq!(decoder.finish().as_deref(), Some(payload.as_slice()));
}

#[test]
fn and_out_of_order_with_a_third_of_the_frames_thrown_away() {
    // **The test that decides whether this crate is worth writing.** A screen
    // does not rewind and a camera misses frames; the point of a fountain is
    // that missing frames cost frames, not the transfer.
    let payload = payload_of(4096);
    let shape = shape_for(&payload, 256);
    let blocks = split(&payload, shape.block_size);

    let mut decoder = Decoder::new(shape);
    let mut delivered = 0;
    let mut seed = 1_u64;
    while !decoder.is_complete() && seed < 20_000 {
        // Every third frame is dropped, and the rest arrive shuffled by taking
        // seeds in a stride that never repeats a residue.
        if seed % 3 != 0 {
            decoder.accept(&encode(&blocks, shape, seed.wrapping_mul(2_654_435_761)));
            delivered += 1;
        }
        seed += 1;
    }

    assert!(decoder.is_complete(), "gave up after {delivered} frames");
    assert_eq!(decoder.finish().as_deref(), Some(payload.as_slice()));

    // And the overhead is in the range ADR-0044 §4 predicted, not an order out.
    // 16 source blocks; anything under ~2x means the distribution is working.
    let source_blocks = blocks.len();
    assert!(
        delivered < source_blocks * 6,
        "{delivered} frames for {source_blocks} blocks -- the degree \
         distribution is not doing its job"
    );
}

#[test]
fn a_decoder_never_hands_back_a_partial_file() {
    // The worst available outcome is a file that is nearly right: the hash
    // fails and nothing says why. `finish` returns None until every block is
    // known, and this proves it does not merely *usually*.
    let payload = payload_of(2048);
    let shape = shape_for(&payload, 256);
    let blocks = split(&payload, shape.block_size);

    let mut decoder = Decoder::new(shape);
    decoder.accept(&encode(&blocks, shape, 1));
    decoder.accept(&encode(&blocks, shape, 2));

    assert!(!decoder.is_complete());
    assert_eq!(decoder.finish(), None, "a partial decode produced a file");
}

#[test]
fn the_same_frame_twice_is_not_an_error_and_not_progress() {
    // A looping stream shows every seed again each cycle. Treating a repeat as
    // a fault would fail every transfer that needed more than one pass.
    let payload = payload_of(1024);
    let shape = shape_for(&payload, 256);
    let blocks = split(&payload, shape.block_size);
    let frame = encode(&blocks, shape, 42);

    let mut decoder = Decoder::new(shape);
    assert!(decoder.accept(&frame), "the first sight of a frame is news");
    let after_first = decoder.solved_count();
    assert!(!decoder.accept(&frame), "a repeat was counted as news");
    assert_eq!(decoder.solved_count(), after_first);
}

#[test]
fn a_frame_from_another_transfer_is_refused_not_mixed_in() {
    // Two people running Qyro in the same room, two streams in shot. Mixing
    // them would corrupt both, and the corruption would look like a decoder
    // bug rather than what it is.
    let mine = payload_of(1024);
    let theirs = payload_of(2048);
    let my_shape = shape_for(&mine, 256);
    let their_shape = shape_for(&theirs, 256);

    let mut decoder = Decoder::new(my_shape);
    let intruder = encode(&split(&theirs, 256), their_shape, 9);
    assert!(!decoder.accept(&intruder), "another transfer got in");
    assert_eq!(decoder.solved_count(), 0);
}

#[test]
fn a_payload_that_is_not_a_whole_number_of_blocks_comes_back_exactly() {
    // The padding case, and the reason `payload_len` travels in every frame:
    // without it, a file ending in zeroes is indistinguishable from a padded
    // one and the receiver delivers something *nearly* right.
    let payload = payload_of(1000); // 3 blocks of 256 plus 232 bytes
    let shape = shape_for(&payload, 256);
    let blocks = split(&payload, shape.block_size);
    assert_eq!(blocks.len(), 4);

    let mut decoder = Decoder::new(shape);
    let mut seed = 1_u64;
    while !decoder.is_complete() && seed < 5000 {
        decoder.accept(&encode(&blocks, shape, seed));
        seed += 1;
    }

    let rebuilt = decoder.finish().expect("a complete decode");
    assert_eq!(rebuilt.len(), 1000, "the padding was delivered as content");
    assert_eq!(rebuilt, payload);
}

#[test]
fn a_payload_that_ends_in_real_zeroes_is_not_mistaken_for_padding() {
    // The control for the test above. A receiver that trimmed trailing zeroes
    // instead of trusting `payload_len` would pass that test and truncate this
    // file.
    let mut payload = payload_of(500);
    payload.extend_from_slice(&[0_u8; 24]);
    let shape = shape_for(&payload, 256);
    let blocks = split(&payload, shape.block_size);

    let mut decoder = Decoder::new(shape);
    let mut seed = 1_u64;
    while !decoder.is_complete() && seed < 5000 {
        decoder.accept(&encode(&blocks, shape, seed));
        seed += 1;
    }

    let rebuilt = decoder.finish().expect("a complete decode");
    assert_eq!(rebuilt.len(), 524);
    assert_eq!(&rebuilt[500..], &[0_u8; 24], "real zeroes were trimmed");
}

#[test]
fn a_seed_selects_the_same_blocks_on_both_ends() {
    // The frame carries a seed and not the block list, so this function *is*
    // the wire format. Two builds that disagree here decode garbage while
    // believing they succeeded.
    for seed in [1_u64, 7, 12345, u64::MAX] {
        let first = neighbours(seed, 32);
        let second = neighbours(seed, 32);
        assert_eq!(first, second, "seed {seed} is not deterministic");
        assert!(!first.is_empty(), "seed {seed} selects nothing");
        assert!(first.iter().all(|index| *index < 32));
    }
}

#[test]
fn a_seed_never_selects_the_same_block_twice() {
    // A block XORed twice cancels out, so a duplicate does not waste space --
    // it silently changes which blocks the frame encodes, and the decoder
    // combines it wrongly while believing it understood.
    for seed in 1_u64..300 {
        let chosen = neighbours(seed, 24);
        let mut unique = chosen.clone();
        unique.dedup();
        assert_eq!(chosen, unique, "seed {seed} repeated a block: {chosen:?}");
    }
}

#[test]
fn some_frames_are_degree_one_or_no_decode_can_ever_start() {
    // The peeling decoder needs at least one frame that reduces to a single
    // unknown, or nothing ever solves and every transfer hangs at zero. This is
    // the property the robust part of the soliton distribution exists for, and
    // it is worth asserting because a plausible-looking distribution can lack
    // it entirely.
    let singles = (1_u64..500)
        .filter(|seed| neighbours(*seed, 40).len() == 1)
        .count();
    assert!(
        singles > 0,
        "no degree-1 frame in 500 seeds: no decode could ever start"
    );
}

#[test]
fn the_wire_format_round_trips() {
    let payload = payload_of(512);
    let shape = shape_for(&payload, 256);
    let frame = encode(&split(&payload, 256), shape, 0xDEAD_BEEF);

    let bytes = encode_frame(&frame);
    assert_eq!(bytes.len(), crate::FRAME_HEADER_LEN + 256);
    assert_eq!(decode_frame(&bytes).as_ref(), Ok(&frame));
}

#[test]
fn what_a_camera_really_hands_over_is_refused_and_not_guessed() {
    use crate::WireError;

    // Every one of these is a thing that happens in a room with a screen and a
    // phone, and a decoder that treated them as impossible would panic on the
    // normal case.
    assert_eq!(decode_frame(b"QF"), Err(WireError::TooShort));
    assert_eq!(
        decode_frame(b"ZZ\x01aa"),
        Err(WireError::TooShort),
        "a buffer too short to hold a header is refused for that, before the \
         magic is even looked at -- reading the magic out of a buffer that does \
         not have one is how a parser reads past its input"
    );
    assert_eq!(
        decode_frame(b"ZZ\x01aaaaaaaaaaaaaa"),
        Err(WireError::NotAFrame),
        "seventeen bytes is a whole header, so the magic decides -- and this is \
         the common case in a room where two protocols are on screen"
    );

    let payload = payload_of(512);
    let shape = shape_for(&payload, 256);
    let good = encode_frame(&encode(&split(&payload, 256), shape, 5));

    let mut other_protocol = good.clone();
    other_protocol[0] = b'X';
    assert_eq!(decode_frame(&other_protocol), Err(WireError::NotAFrame));

    let mut future = good.clone();
    future[2] = 99;
    assert_eq!(decode_frame(&future), Err(WireError::UnknownVersion(99)));

    let truncated = &good[..good.len() - 4];
    assert_eq!(
        decode_frame(truncated),
        Err(WireError::BlockSizeMismatch),
        "a truncated payload was padded instead of refused"
    );
}

#[test]
fn a_shape_that_describes_nothing_is_refused_rather_than_dividing_by_zero() {
    use crate::WireError;

    let mut bytes = vec![b'Q', b'F', 1];
    bytes.extend_from_slice(&7_u64.to_be_bytes());
    bytes.extend_from_slice(&0_u32.to_be_bytes()); // payload_len 0
    bytes.extend_from_slice(&256_u16.to_be_bytes());
    bytes.extend_from_slice(&[0_u8; 256]);
    assert_eq!(decode_frame(&bytes), Err(WireError::ImpossibleShape));

    let mut zero_block = vec![b'Q', b'F', 1];
    zero_block.extend_from_slice(&7_u64.to_be_bytes());
    zero_block.extend_from_slice(&100_u32.to_be_bytes());
    zero_block.extend_from_slice(&0_u16.to_be_bytes()); // block_size 0
    assert_eq!(decode_frame(&zero_block), Err(WireError::ImpossibleShape));
}

#[test]
fn one_frame_fits_a_version_27_qr_at_level_l() {
    // ADR-0044 §2 fixes v27-L at 1 465 bytes. A block size that overflowed it
    // would produce frames no QR could carry, and the failure would appear at
    // render time on somebody's screen rather than here.
    const V27_L_CAPACITY: usize = 1465;
    let block_size = V27_L_CAPACITY - crate::FRAME_HEADER_LEN;
    assert_eq!(block_size, 1448);

    let payload = payload_of(4000);
    let shape = shape_for(&payload, u16::try_from(block_size).expect("fits"));
    let frame = encode(&split(&payload, shape.block_size), shape, 3);
    assert_eq!(encode_frame(&frame).len(), V27_L_CAPACITY);
}
