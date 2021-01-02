use cursive::views::TextContent;
use viste_reactive::RWire;

pub fn appending(text: &TextContent) -> RWire<String> {
    let text = text.clone();
    RWire::new(move |s| text.append(s))
}

pub fn setting(text: &TextContent) -> RWire<String> {
    let text = text.clone();
    RWire::new(move |s| text.set_content(s))
}
