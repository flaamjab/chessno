use std::{
    fs::OpenOptions,
    io::{ErrorKind, Write},
    path::Path,
};

use spirv_compiler::{CompilerBuilder, CompilerError, ShaderKind};

fn main() {
    let source_folder = Path::new("./shaders");
    let target_folder = Path::new("./assets/shaders");
    if let Err(e) = std::fs::create_dir(target_folder) {
        match e.kind() {
            ErrorKind::AlreadyExists => {}
            _ => std::process::exit(1),
        }
    }
    for entry in source_folder.read_dir().unwrap() {
        let source_file = entry.unwrap().path();
        if source_file.metadata().unwrap().is_file() {
            let mut target_name = source_file.file_stem().unwrap().to_os_string();
            target_name.push(".");
            target_name.push(
                source_file
                    .extension()
                    .expect("source file must have an extension (e.g. \".frag\""),
            );
            target_name.push(".spv");
            let target_file = target_folder.join(target_name);

            let maybe_code = compile_shader(&source_file);
            match maybe_code {
                Ok(code) => {
                    write_shader(&target_file, &code);
                }
                Err(e) => {
                    eprintln!("{}: {}", source_folder.display(), e)
                }
            }
        }
    }
}

fn compile_shader(path: &Path) -> Result<Vec<u32>, CompilerError> {
    let mut compiler = CompilerBuilder::new().build().unwrap();
    let extension = path
        .extension()
        .expect("a shader source file must have and extension (e.g. \".frag\"");

    let kind = if extension == "vert" {
        ShaderKind::Vertex
    } else if extension == "frag" {
        ShaderKind::Fragment
    } else {
        return Err(CompilerError::LoadError(
            "Unsupported shader type".to_string(),
        ));
    };

    compiler.compile_from_file(path, kind, true)
}

fn write_shader(path: &Path, shader: &[u32]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&path)
        .expect("make sure the parent folder exists");

    unsafe {
        let shader: Vec<u8> = shader
            .iter()
            .flat_map(|n| {
                let bytes: [u8; 4] = std::mem::transmute(*n);
                bytes
            })
            .collect();
        file.write(&shader).unwrap();
    }
}
