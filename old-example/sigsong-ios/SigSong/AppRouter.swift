//
//  AppRouter.swift
//  SigSong
//
//  Created by Codex on 2024/10/31.
//

import Foundation

enum AppDestination: Hashable {
    case lyric(id: Int32)
}

@MainActor
final class AppRouter: ObservableObject {
    @Published var path: [AppDestination] = []

    func push(_ destination: AppDestination) {
        path.append(destination)
    }

    func openLyric(id: Int32) {
        push(.lyric(id: id))
    }

    func reset() {
        path.removeAll()
    }
}
