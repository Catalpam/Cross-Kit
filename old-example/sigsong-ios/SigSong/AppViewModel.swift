//
//  AppViewModel.swift
//  SigSong
//
//  Created by zigengm3 on 2024/8/2.
//

import Combine
import Foundation
import InvokeKit
import SigsongSDK

@MainActor
final class AppViewModel: ObservableObject {
    private var cancellables: Set<AnyCancellable> = []

    @Published var router = AppRouter()
    @Published var userStore = UserStore()
    @Published var toast = NTToast()

    init() {
        listenFFIPush()
    }

    private func listenFFIPush() {
        EventPublisher
            .sink { [weak self] event in
                guard let self else { return }
                switch event {
                case .showToast(let toastType, let message):
                    self.toast.showToast(toastType, message: message)
                }
            }
            .store(in: &cancellables)
    }
}
