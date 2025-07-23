use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=assets/shaders");

    for entry in fs::read_dir("assets/shaders").unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            let name_owned = entry.file_name();
            let name = name_owned.to_str().unwrap();
            wesl::Wesl::new("assets/shaders").build_artefact(
                "pipeline/".to_string() + name,
                name.trim_end_matches(".wgsl"),
            );
        }
    }
}
