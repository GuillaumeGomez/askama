use rinja::Template;

#[derive(Template)]
#[template(
    source = "🙂",
    ext = "txt"
)]
#[template(
    source = "🙃",
    ext = "txt"
)]
struct TwoEmojis;

fn main() {
}
