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
