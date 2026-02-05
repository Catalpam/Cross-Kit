//
// TabBar.swift
// swiftuidemo
//
// Created by Zigeng on 2024/1/25.
//

import Foundation
import SwiftUI

struct NTTabView<Content: View>: View {
    @Environment(\.horizontalSizeClass) var sizeClass

    let content: Content
    init(@ViewBuilder content: () -> Content) {
        self.content = content()
        //    let appearance = UITabBarAppearance()
        //    appearance.backgroundColor = UIColor.gray

        // 更改选中项的颜色
        //    UITabBar.appearance().backgroundColor = UIColor.gray
        //    UITabBar.appearance().tintColor = .green
        //    UITabBar.appearance().unselectedItemTintColor = .yellow
        //    UITabBar.appearance().standardAppearance = appearance
        //    UITabBar.appearance().scrollEdgeAppearance = appearance
    }

    var body: some View {
        if sizeClass == .compact {
            TabView {
                content
            }
            .tabViewStyle(.tabBarOnly)
        } else {
            TabView {
                content
                    .environment(\.horizontalSizeClass, sizeClass) // 避免未知行为
            }
            .environment(\.horizontalSizeClass, .compact) // Use this modifier 来避免iOS18的新界面
            .tabViewStyle(.tabBarOnly)
//            .navigationViewStyle(StackNavigationViewStyle()) // 使用StackNavigationViewStyle
        }
    }

}
