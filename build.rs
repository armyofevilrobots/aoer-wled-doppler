use std::{
    env,
    error::Error,
    fs::{self, File},
    io::Write,
    path::Path,
};

const SOURCE_DIR: &str = "webview/aoer-wled-doppler-web/dist/";

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = "src";
    let dest_path = Path::new(&out_dir).join("webui_content.rs");
    let mut all_the_files = File::create(&dest_path)?;

    writeln!(&mut all_the_files, r##"["##,)?;

    for f in fs::read_dir(SOURCE_DIR)? {
        let f = f?;

        if !f.file_type()?.is_file() {
            continue;
        }

        writeln!(
            &mut all_the_files,
            r##"("{name}", Vec::<u8>::from(include_bytes!(r#"../{name}"#))),"##,
            // r##"("{name}", include_bytes!(r#"../{name}"#)),"##,
            name = f.path().display(),
        )?;
    }

    writeln!(&mut all_the_files, r##"]"##,)?;

    Ok(())
}
