fn main() {
    println!("cargo:rerun-if-changed=src/renderer/pipeline/shaders");

    wesl::Wesl::new("src/renderer/pipeline/shaders").build_artefact("main", "main");
    wesl::Wesl::new("src/renderer/pipeline/shaders").build_artefact("texture", "texture");
    wesl::Wesl::new("src/renderer/pipeline/shaders").build_artefact("background", "background");
    wesl::Wesl::new("src/renderer/pipeline/shaders").build_artefact("hdr", "hdr");
}
