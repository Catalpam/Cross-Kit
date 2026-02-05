//
//  SongLyricScreen.swift
//  SigSong
//
//  Created by Bill Haku on 2024/8/4.
//

import AVFoundation
import SwiftUI
import SigsongSDK
import InvokeKit
import SFSafeSymbols

fileprivate class SongLyricScreenViewModel: ObservableObject {
    @Published var song: ClSongInfo?
    @Published var playingIndex: UInt32 = 0

    init(id: Int32) {
        Task {
            await getSongInfo(id: id)
        }
    }

    func getSongInfo(id: Int32) async {
        do {
            let song = try await API.getSongById(id: id)
            Task { @MainActor in
                self.song = song
            }
            print(song)
        } catch {
            print("拉取歌词错误")
        }
    }
}

struct SongLyricScreen: View {
    @State private var player = AVPlayer()
    @StateObject private var vm: SongLyricScreenViewModel
    init(id: Int32) {
        self._vm = .init(wrappedValue: SongLyricScreenViewModel(id: id))
    }
    let timer = Timer.publish(every: 0.5, on: .main, in: .common).autoconnect()
    var body: some View {
        // 主内容
        ScrollViewReader { reader in
        ScrollView {
            VStack {
//                    // TODO: 这段可以删一下
                if let song = vm.song {
                    Text(song.title)
                        .font(.headline)
                        .padding(.bottom)
                    LyricsView(lyrics: song.lyrics)
                }
            }
            .padding(.bottom, 120)
            //            .onReceive(timer) { _ in
            //                if let song = song, player.timeControlStatus == .playing {
                //                    for lyric in song.lyrics {
                //                        if Double(lyric.startTime) / 1000 > player.currentTime().seconds {
                //                            withAnimation{
                //                                playingIndex = lyric.startTime
                //                                reader.scrollTo(lyric.startTime, anchor: .center)
                //                            }
                //                            return
                //                        }
                //                    }
                //                }
//                            }
            }
            .navigationTitle(vm.song?.title ?? "")
                    .task {
//                        song = await API.getSong(id: songId)
//                        if let musicPath = song?.musicPath {
//                            if let url = URL(string: musicPath) {
//                                let playerItem = AVPlayerItem(url: url)
//                                player.replaceCurrentItem(with: playerItem)
//                            }
//                        }
                    }
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button(action: {
                        if player.timeControlStatus == .playing {
                            player.pause()
                        } else {
                            player.seek(to: .zero)
                            player.play()
                        }
                    }, label: {
                        Image(systemSymbol: .musicNote)
                    })
                }
            }
        }
    }
}
