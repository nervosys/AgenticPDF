//! Decode a JPEG 2000 file named by `APDF_JPX` and write it out as a PPM.
//!
//! Ignored by default: it is a hand-driven probe for looking at a real
//! codestream, not an assertion about one.

#[test]
#[ignore]
fn decode_a_jpx_file() {
    let Ok(path) = std::env::var("APDF_JPX") else {
        eprintln!("set APDF_JPX to a .jp2 or .j2k file");
        return;
    };
    let data = std::fs::read(&path).expect("read the file");
    let start = std::time::Instant::now();
    let image = agenticpdf::image::jpx::decode(&data).expect("decode");
    eprintln!(
        "{}x{} x{} in {:?}",
        image.width,
        image.height,
        image.components,
        start.elapsed()
    );
    let out = std::env::var("APDF_JPX_OUT").unwrap_or_else(|_| format!("{path}.ppm"));
    let mut ppm = format!("P6\n{} {}\n255\n", image.width, image.height).into_bytes();
    for p in image.data.chunks(image.components) {
        if image.components >= 3 {
            ppm.extend_from_slice(&p[..3]);
        } else {
            ppm.extend_from_slice(&[p[0], p[0], p[0]]);
        }
    }
    std::fs::write(&out, ppm).expect("write");
    eprintln!("wrote {out}");
}

/// Decode a raw JBIG2 embedded stream named by `APDF_JBIG2` and write it as
/// a PBM. Ignored by default, for the same reason as the JPX probe.
#[test]
#[ignore]
fn decode_a_jbig2_file() {
    let Ok(path) = std::env::var("APDF_JBIG2") else {
        eprintln!("set APDF_JBIG2, APDF_W and APDF_H");
        return;
    };
    let w: usize = std::env::var("APDF_W").unwrap().parse().unwrap();
    let h: usize = std::env::var("APDF_H").unwrap().parse().unwrap();
    let data = std::fs::read(&path).expect("read the file");
    let start = std::time::Instant::now();
    let bitmap = agenticpdf::image::jbig2::decode_embedded(&[], &data, w, h).expect("decode");
    let set = bitmap.bits.iter().filter(|&&b| b != 0).count();
    eprintln!(
        "{}x{} in {:?}, {:.1}% set",
        bitmap.w,
        bitmap.h,
        start.elapsed(),
        100.0 * set as f64 / bitmap.bits.len() as f64
    );
    let mut ppm = format!("P6\n{} {}\n255\n", bitmap.w, bitmap.h).into_bytes();
    for &b in &bitmap.bits {
        let v = if b != 0 { 0u8 } else { 255 };
        ppm.extend_from_slice(&[v, v, v]);
    }
    std::fs::write(format!("{path}.ppm"), ppm).expect("write");
    eprintln!("wrote {path}.ppm");
}
