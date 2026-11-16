// Shared by crates/astur/build.rs and crates/astur-settings/build.rs
// (`include!`d). Embeds the icon and a VERSIONINFO block built from the
// crate's Cargo.toml (embed-resource), and assets/astur.manifest
// (embed-manifest: link args on MSVC so link.exe's own manifest can't make a
// duplicate, a COFF object on GNU). An unsigned exe with no version info or
// manifest that installs global input hooks is a common AV "unwanted
// software" (PUA) verdict. The .rc is generated into OUT_DIR so the version
// can't drift from Cargo.toml; paths are absolute with forward slashes, which
// both rc.exe and windres accept.

fn embed_resources(name: &str, description: &str) {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../astur/assets")
        .canonicalize()
        .unwrap();
    let assets = assets.to_string_lossy().trim_start_matches(r"\\?\").replace('\\', "/");
    let v = std::env::var("CARGO_PKG_VERSION").unwrap();
    let num = |key: &str| std::env::var(key).unwrap().parse::<u16>().unwrap_or(0);
    let (a, b, c) = (
        num("CARGO_PKG_VERSION_MAJOR"),
        num("CARGO_PKG_VERSION_MINOR"),
        num("CARGO_PKG_VERSION_PATCH"),
    );
    let rc = format!(
        r#"1 ICON "{assets}/astur.ico"
1 VERSIONINFO
FILEVERSION {a},{b},{c},0
PRODUCTVERSION {a},{b},{c},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "Astur"
      VALUE "FileDescription", "{description}"
      VALUE "FileVersion", "{v}"
      VALUE "InternalName", "{name}"
      VALUE "LegalCopyright", "Licensed under Apache-2.0"
      VALUE "OriginalFilename", "{name}.exe"
      VALUE "ProductName", "Astur"
      VALUE "ProductVersion", "{v}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    );
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join(format!("{name}.rc"));
    std::fs::write(&out, rc).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={assets}/astur.ico");
    println!("cargo:rerun-if-changed={assets}/astur.manifest");
    println!("cargo:rerun-if-changed={assets}/../resources.rs");
    embed_resource::compile(&out, embed_resource::NONE);
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_manifest::embed_manifest_file(format!("{assets}/astur.manifest"))
            .expect("unable to embed assets/astur.manifest");
    }
}
