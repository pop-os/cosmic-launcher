use std::path::PathBuf;
use std::{env, fs};
use xdgen::{App, Context, FluentString};

fn main() {
    println!("cargo:rerun-if-env-changed=APP_ID");
    let id = env::var("APP_ID").unwrap_or_else(|_| String::from("com.system76.CosmicLauncher"));
    let ctx = Context::new("i18n", env::var("CARGO_PKG_NAME").unwrap()).unwrap();
    let app = App::new(FluentString("xdg-name"))
        .comment(FluentString("xdg-comment"))
        .keywords(FluentString("xdg-keywords"));
    let output = PathBuf::from("target/xdgen");
    fs::create_dir_all(&output).unwrap();
    fs::write(
        output.join(format!("{}.desktop", id)),
        app.expand_desktop(format!("data/{}.desktop", id), &ctx)
            .unwrap(),
    )
    .unwrap();
    fs::write(
        output.join(format!("{}.metainfo.xml", id)),
        app.expand_metainfo(format!("data/{}.metainfo.xml", id), &ctx)
            .unwrap(),
    )
    .unwrap();
}
