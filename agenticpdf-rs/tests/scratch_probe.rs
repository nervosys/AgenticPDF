#[test]
#[ignore]
fn probe() {
    let data = std::fs::read(std::env::var("APDF_PDF").unwrap()).unwrap();
    let doc = agenticpdf::engine::Document::parse(&data).unwrap();
    let mut ok = 0;
    let mut bad = 0;
    for num in doc.object_numbers() {
        let obj = doc.resolve(&agenticpdf::engine::Object::Ref(num, 0));
        let agenticpdf::engine::Object::Stream(d, raw) = &obj else { continue };
        match agenticpdf::engine::decode_stream(d, raw) {
            Ok(v) => {
                ok += 1;
                let _ = v;
            }
            Err(e) => {
                bad += 1;
                eprintln!("BAD obj {num}: raw {} -> {e:?}", raw.len());
            }
        }
    }
    eprintln!("streams: {ok} ok, {bad} failed");
}
