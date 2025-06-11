fn main() {
    let path = std::env::args().nth(1).expect("Please provide a file path");
    let f = std::fs::File::open(path).expect("Opening file");
    let b = std::io::BufReader::new(f);
    let resources = cfn_lsp_schema::extract_from_bundle(b).expect("Extracting from bundle");
    dbg!(resources);
}
