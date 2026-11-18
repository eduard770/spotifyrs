fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles);
    slint_build::compile_with_config("ui/app.slint", config)
        .expect("Não foi possível compilar a interface Slint");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=ui/spotifyrs.rc");
        println!("cargo:rerun-if-changed=ui/assets/spotifyrs.ico");
        embed_resource::compile("ui/spotifyrs.rc", embed_resource::NONE)
            .manifest_required()
            .expect("Não foi possível incorporar o ícone do SpotifyRS");
    }
}
