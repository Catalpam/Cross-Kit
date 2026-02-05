use sig_song_sdk as sdk;

fn optional_env_var(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

#[tokio::test]
async fn sdk_requests_real_server() {
    let word = optional_env_var("SIGSONG_TEST_WORD", "描いた");
    let song_id_raw = optional_env_var("SIGSONG_TEST_SONG_ID", "61301499");
    let song_id: i32 = song_id_raw.parse().expect("SIGSONG_TEST_SONG_ID must be a valid i32");

    let base_url = optional_env_var("SIGSONG_SERVER_BASE_URL", "http://127.0.0.1:7878/api");

    // let word_info_resp = sdk::server::api::word_info::get_word_info(word.clone())
    //     .await
    //     .unwrap_or_else(|err| {
    //         panic!(
    //             "failed to fetch word info from sigsong-server at {base_url}; start the server and ensure test data exists: {err}"
    //         )
    //     });
    // assert!(
    //     !word_info_resp.word_infos.is_empty(),
    //     "word info response should contain entries for {word}"
    // );

    let song_resp = sdk::server::api::song::get_song_by_id(song_id)
        .await
        .unwrap_or_else(|err| {
            panic!(
                "failed to fetch song id {song_id} from sigsong-server at {base_url}; ensure the song exists in MongoDB: {err}"
            )
        });
    let song = song_resp.song.expect(
        "song payload absent in GetSongByIdResponse; ensure database contains the requested song",
    );

    print!("{song:?}");
    assert_eq!(song.id, song_id, "song id mismatch");

    let recommend_resp = sdk::server::api::song::get_song_recommend(0)
        .await
        .unwrap_or_else(|err| {
            panic!(
                "failed to fetch recommendations from sigsong-server at {base_url}; ensure recommendation endpoint is available: {err}"
            )
        });
    assert!(
        !recommend_resp.briefs.is_empty(),
        "recommendation response should include at least one song"
    );

    let search_keyword = song.title.clone();
    let search_resp = sdk::server::api::song::search_song(search_keyword.clone())
        .await
        .unwrap_or_else(|err| {
            panic!(
                "failed to search song '{search_keyword}' from sigsong-server at {base_url}; ensure search endpoint is implemented: {err}"
            )
        });
    assert!(
        search_resp.briefs.iter().any(|brief| brief.id == song_id),
        "search response should contain the fetched song id"
    );

    let login_resp = sdk::server::api::passport::login("demo@sigsong", "sigsong123")
        .await
        .unwrap_or_else(|err| {
            panic!(
                "failed to login via sigsong-server at {base_url}; ensure login endpoint is available: {err}"
            )
        });
    let user = login_resp.user.expect("login response should contain user payload");
    assert_eq!(user.name, "SigSong Demo");
    assert!(!login_resp.token.is_empty(), "login response token should not be empty");
}
