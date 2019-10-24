use pulldown_cmark::{html, Options, Parser};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn render(input: &str) -> String {
    let parser = Parser::new_ext(input, Options::empty());
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    return html_output;
}
