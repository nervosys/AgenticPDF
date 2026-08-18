#[test]
#[ignore]
fn dump() {
    let data = std::fs::read(std::env::var("APDF_PDF").unwrap()).unwrap();
    let page: usize = std::env::var("APDF_PAGE").unwrap().parse().unwrap();
    let content = agenticpdf::engine::page_content_bytes(&data, page).unwrap_or_default();
    std::fs::write(std::env::var("APDF_OUT").unwrap(), &content).unwrap();
    eprintln!("{} bytes", content.len());
}
