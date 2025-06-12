fn main() {
    let output_path = std::env::args().nth(1).expect("Please provide a file path");
    cfn_lsp_schema::render_to(&output_path).unwrap();
    println!("Schema rendered to {output_path}");
}
