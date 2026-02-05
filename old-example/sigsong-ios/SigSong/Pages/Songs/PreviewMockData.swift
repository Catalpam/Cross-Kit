//
//  PreviewMockData.swift
//  SigSong
//
//  Created by zigengm3 on 2025/9/28.
//
import SigsongSDK

enum MockData {
    static let song = ClSongInfo(
        id: 61301499,
        title: "spiral",
        lyrics: [
            // ===== 第1行 =====
            SigsongSDK.ClSongLyric(
                id: 1,
                text: "描いた地図は引き裂いた",
                zhCn: "昔日描绘的地图已破旧不堪",
                elements: [
                    SigsongSDK.ClSongLyricElement(
                        id: 11,
                        surface: "描い",
                        wordType: .verb,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "描", pronunciation: "えが"),
                            SigsongSDK.PronouncedString(original: "い", pronunciation: nil)
                        ],
                        ruby: "egai"
                    ),
                    SigsongSDK.ClSongLyricElement(
                        id: 12,
                        surface: "た",
                        wordType: .auxiliaryVerb,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "た", pronunciation: nil)
                        ],
                        ruby: "ta"
                    ),
                    SigsongSDK.ClSongLyricElement(
                        id: 13,
                        surface: "地図",
                        wordType: .noun,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "地図", pronunciation: "ちず")
                        ],
                        ruby: "chizu"
                    ),
                    SigsongSDK.ClSongLyricElement(
                        id: 14,
                        surface: "は",
                        wordType: .particle,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "は", pronunciation: nil)
                        ],
                        ruby: "wa"
                    ),
                    SigsongSDK.ClSongLyricElement(
                        id: 15,
                        surface: "引き裂い",
                        wordType: .verb,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "引", pronunciation: "ひ"),
                            SigsongSDK.PronouncedString(original: "き", pronunciation: nil),
                            SigsongSDK.PronouncedString(original: "裂", pronunciation: "さ"),
                            SigsongSDK.PronouncedString(original: "い", pronunciation: nil)
                        ],
                        ruby: "hikisa"
                    ),
                    SigsongSDK.ClSongLyricElement(
                        id: 16,
                        surface: "た",
                        wordType: .auxiliaryVerb,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "た", pronunciation: nil)
                        ],
                        ruby: "ta"
                    )
                ]
            ),

            // ===== 第2行 =====
            SigsongSDK.ClSongLyric(
                id: 2,
                text: "世界はあの日のまま",
                zhCn: "世界却仍是一如那日的模样",
                elements: [
                    SigsongSDK.ClSongLyricElement(
                        id: 21,
                        surface: "世界",
                        wordType: .noun,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "世界", pronunciation: "せかい")
                        ],
                        ruby: "sekai"
                    ),
                    SigsongSDK.ClSongLyricElement(
                        id: 22,
                        surface: "は",
                        wordType: .particle,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "は", pronunciation: nil)
                        ],
                        ruby: "wa"
                    ),
                    SigsongSDK.ClSongLyricElement(
                        id: 23,
                        surface: "あの",
                        wordType: .pronoun,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "あの", pronunciation: nil)
                        ],
                        ruby: "ano"
                    ),
                    SigsongSDK.ClSongLyricElement(
                        id: 24,
                        surface: "日",
                        wordType: .noun,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "日", pronunciation: "ひ")
                        ],
                        ruby: "hi"
                    ),
                    SigsongSDK.ClSongLyricElement(
                        id: 25,
                        surface: "の",
                        wordType: .particle,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "の", pronunciation: nil)
                        ],
                        ruby: "no"
                    ),
                    SigsongSDK.ClSongLyricElement(
                        id: 26,
                        surface: "まま",
                        wordType: .adverb,
                        pronouncedString: [
                            SigsongSDK.PronouncedString(original: "まま", pronunciation: nil)
                        ],
                        ruby: "mama"
                    )
                ]
            ),
        ]
    )
}
