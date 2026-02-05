//
//  SigSongApp.swift
//  SigSong
//
//  Created by ZigengM1 on 2023/11/05.
//

import SwiftUI

@main
struct SigSongApp: App {
    @StateObject private var appViewModel = AppViewModel()

    var body: some Scene {
        WindowGroup {
            NavigationStack(path: $appViewModel.router.path) {
                ContentView()
                    .navigationDestination(for: AppDestination.self) { destination in
                        switch destination {
                        case .lyric(let id):
                            SongLyricScreen(id: id)
                        }
                    }
            }
            .environmentObject(appViewModel.router)
            .environmentObject(appViewModel.userStore)
            .environmentObject(appViewModel.toast)
            .wrapToast(message: appViewModel.toast.toastMessage)
        }
    }
}
