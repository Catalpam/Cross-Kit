//
// NTSideBar.swift
// swiftuidemo
//
// Created by Zigeng on 2024/1/25.
//

import SwiftUI

struct NTSideBar: View {
    @Binding var isSidebarVisible: Bool
    @Binding var sidebarOffset: CGFloat
    let sidebarWidth: CGFloat = 250

    var dragGesture: some Gesture {
        FollowDragGesture()
            .bindOffset($sidebarOffset)
            .maxLimit(0)
            .minLimit(-sidebarWidth)
            .minEnd(closeByQuickPan)
    }

    var sideBar: some View {
        ZStack {
            Color.gray
            VStack {
                NTAvatar()
                List {
                    NavigationLink("Home", destination: Text("Home View"))
                    NavigationLink("Settings", destination: Text("Settings View"))
                    // 更多的链接或按钮
                }
            }
            .padding(.top, 50)
        }
        .frame(width: sidebarWidth)
        .offset(x: sidebarOffset)
    }

    var backGroundAlpha: CGFloat {
        (sidebarWidth + sidebarOffset) / sidebarWidth * 0.7
    }

    var body: some View {
        ZStack {
            if isSidebarVisible {
                Color.gray
                    .opacity(backGroundAlpha)
                    .edgesIgnoringSafeArea(.all)
                    .onTapGesture(perform: closeSideBar)
            }
            HStack {
                sideBar.gesture(dragGesture)
                Spacer()
            }
        }.ignoresSafeArea(.all)
    }
}

extension NTSideBar {
    func closeSideBar() {
        closeSideBar(0.2)
    }

    func closeSideBar(_ duration: TimeInterval) {
        NTUI.withAnimation(.easeOut(duration: duration), completeDuration: duration, {
            sidebarOffset = -sidebarWidth
        }, completion: {
            isSidebarVisible = false
        })
    }
    func closeByQuickPan(_ velocity: CGFloat) {
        let duration = min(abs(sidebarOffset / velocity) * 5, 0.2)
        closeSideBar(duration)
    }
}

public struct NTUI {
    public static func withAnimation<Result>(_ animation: Animation? = .default,
                                             completeDuration: TimeInterval = 0.2,
                                             _ body: () throws -> Result,
                                             completion: @escaping () -> Void) rethrows -> Result {
        if #available(iOS 17.0, *) {
            return try SwiftUI.withAnimation(animation, body, completion: completion)
        } else {
            let result: Result = try SwiftUI.withAnimation(.easeInOut(duration: completeDuration), body)
            DispatchQueue.main.asyncAfter(deadline: .now() + completeDuration) {
                completion()
            }
            return result
        }
    }
}

class SideBarViewModel: ObservableObject {
    let sidebarWidth: CGFloat
    @Published var isSidebarVisible = false
    @Published var sidebarOffset: CGFloat

    init() {
        self.isSidebarVisible = false
        self.sidebarWidth = 250
        self.sidebarOffset = -250
    }

    func toggleSideBarVisible() {
        if isSidebarVisible {
            closeSideBar()
        } else {
            openSideBar()
        }
    }

    func closeSideBar() {
        NTUI.withAnimation(.default,
                           completeDuration: 0.25, {
            sidebarOffset = -sidebarWidth
        }, completion: { [weak self] in
            guard let self = self else { return }
            isSidebarVisible = false
            sidebarOffset = 0
        })
    }

    func openSideBar() {
        sidebarOffset = -250
        isSidebarVisible = true
        withAnimation {
            sidebarOffset = 0
        }
    }
}
