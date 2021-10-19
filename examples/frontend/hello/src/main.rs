use quetta::Text;
use viste_frontend::*;
use viste_reactive::*;
use web_sys::*;

fn main() {
    let world = World::new();
    let window = window().expect("Where is my window?");
    let document = window.document().expect("Where is my document?");
    let main_element = document
        .get_element_by_id("app")
        .expect("Where is my element?");
}
