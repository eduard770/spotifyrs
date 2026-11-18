fn main() {
    // Track the interface and its assets without scanning makepkg's staging directories.
    println!("cargo:rerun-if-changed=ui");
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles);
    slint_build::compile_with_config("ui/app.slint", config)
        .expect("Não foi possível compilar a interface Slint");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("ui/spotifyrs.rc", embed_resource::NONE)
            .manifest_required()
            .expect("Não foi possível incorporar o ícone do SpotifyRS");
    }
}
