// Embed the icon, manifest and version info into astur-settings.exe. Shares
// the WM's script and assets so both exes carry the same hawk and metadata.
include!("../astur/resources.rs");

fn main() {
    embed_resources("astur-settings", "Astur settings");
}
