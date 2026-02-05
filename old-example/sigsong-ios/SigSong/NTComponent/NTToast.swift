//
//  NTToast.swift
//  SigSong
//
//  Created by ZigengM1 on 2024/01/28.
//

import Foundation
import SwiftUI
import SigsongSDK
import Combine

typealias ToastMessage = (ToastType, String)

fileprivate extension ToastType {
    func transform() -> NTIcon.Keys {
        switch self {
        case .info:
            return .info
        case .success:
            return .success
        case .error:
            return .error
        }
    }
}

class NTToastViewModel: ObservableObject {
    @Published var icon: NTIcon.Keys?
    @Published var text: String?
    private var cancellables: Set<AnyCancellable> = []

    init(publisher: PassthroughSubject<ToastMessage?, Never>) {
        publisher.map { msg -> (NTIcon.Keys?, String?) in
            (msg?.0.transform(), msg?.1)
        }.receive(on: DispatchQueue.main).sink { [weak self] (icon, text) in
            guard let self = self else { return }
            withAnimation {
                self.icon = icon
                self.text = text
            }
        }.store(in: &cancellables)
    }
}

struct NTToastView: View {
    @ObservedObject var viewModel: NTToastViewModel
    init(message: PassthroughSubject<ToastMessage?, Never>) {
        self.viewModel = NTToastViewModel(publisher: message)
    }
    var body: some View {
        if let text = viewModel.text, let icon = viewModel.icon {
            HStack {
                Image(systemName: icon.rawValue).foregroundColor(Color.white).padding(.leading, -5)
                Text(text)
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 20)
            .padding(.vertical, 7)
            .background(Color.gray)
            .cornerRadius(.infinity)
        }
    }
}

class NTToast: ObservableObject {
    var toastMessage = PassthroughSubject<ToastMessage?, Never>()
    func showToast(_ type: ToastType, message: String) {
        self.toastMessage.send((type, message))
        DispatchQueue.global().asyncAfter(deadline: .now() + 3) {
            withAnimation(.easeInOut(duration: 0.3)) {
                self.toastMessage.send(nil)
            }
        }
    }
}

extension View {
    func wrapToast(message: PassthroughSubject<ToastMessage?, Never>) -> some View {
        let view = self
        return ZStack {
            view
            VStack {
                Spacer()
                NTToastView(message: message).padding(.bottom, 15)
            }
        }
    }
}

#Preview {
    let vm = PassthroughSubject<ToastMessage?, Never>()
    let view = NTToastView(message: vm)
    vm.send((.error, "用户名密码不太懂"))
    return view
}
