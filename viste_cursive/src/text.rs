use cursive::views::TextContent;
use viste_reactive::RWire;

pub fn appending<'a>(text: TextContent) -> RWire<'a, String> {
    RWire::new(move |s| text.append(s))
}

pub fn setting<'a>(text: TextContent) -> RWire<'a, String> {
    RWire::new(move |s| text.set_content(s))
}
