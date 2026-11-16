// Embed the icon, manifest and version info into astur.exe (see resources.rs).
include!("resources.rs");

fn main() {
    embed_resources("astur", "Astur tiling window manager");
}
