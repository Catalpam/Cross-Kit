fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").map_or(false, |os| os == "ios") {
        println!("cargo:rustc-link-arg=-miphoneos-version-min=18.0");
    }
    server();
}

fn server() {
    let mut config = prost_build::Config::new();
    // 歌曲
    // config.message_attribute("api.GetSongByIDResponse", "#[derive(uniffi::Record)]");
    // config.message_attribute("api.GetSongRecommendResponse", "#[derive(uniffi::Record)]");
    // config.type_attribute(".song", "#[derive(serde::Serialize)]");
    config.type_attribute(".song", "#[derive(sqlx::FromRow)]");
    // config.type_attribute(".word_info", "#[derive(serde::Serialize)]");
    config.type_attribute(".word_info", "#[derive(sqlx::FromRow)]");

    config.out_dir("./src/server/proto");
    config
        .compile_protos(
            &[
                "./sigsong-pb/user.proto",
                "./sigsong-pb/word_info.proto",
                "./sigsong-pb/api.proto",
                "./sigsong-pb/song.proto",
                "./sigsong-pb/passport.proto",
            ],
            &["./sigsong-pb/"],
        )
        .unwrap();
}
