//
//  NT.swift
//  SigSong
//
//  Created by ZigengM1 on 2024/01/26.
//
import Foundation
import SwiftUI
import SigsongSDK

struct ContentView: View {
    var local: Locale {
        return Locale.current
    }

    var sidebar: some View {
        NTSideBar(isSidebarVisible: $vm.isSidebarVisible, sidebarOffset: $vm.sidebarOffset)
    }

    @StateObject var vm = SideBarViewModel()

    @State private var inputSearchText = ""

    @EnvironmentObject var userStore: UserStore
    @EnvironmentObject var router: AppRouter

    @ViewBuilder
    var contentView: some View {
        // 首页
        VStack {
            NTTabView {
                HomeView()
                    .onTapAvatar(vm.toggleSideBarVisible)
                    .tabItem {
                        Label("发现", systemImage: "sparkles")
                    }

                Text("敬请期待")
                    .tabItem {
                        Label("收藏", systemImage: "bookmark")
                    }

                Text("敬请期待")
                    .tabItem {
                        Label("我的", systemImage: "person")
                    }
            }
        }
        //            .tabItem {
        //            }
        //            NavigationView {
        //                VStack {
        //                    SecondView()
        //                    NavigationLink("登陆页面", value: NTNavigationDestination.loginView(account: "nil"))
        //                    Text(userStore.current?.name ?? "haha")
        //                    Text("Main Content Here")
        //                        .navigationBarTitle("Title", displayMode: .inline)
        //                        .navigationBarItems(leading: Button(action: {
        //                            vm.toggleSideBarVisible()
        //                        }, label: {
        //                            Image(systemName: "line.horizontal.3")
        //                        }))
        //                }
        //            }
        //            .tabItem {
        //                Label("设置", systemImage: "gear")
        //            }
        //        }
        //        .onAppear {
        //            print("构造视图耗时 \(Date().timeIntervalSince(last))")
        //            print("登陆总耗时 \(Date().timeIntervalSince(begin))")
        //            last = Date()
        //        }
    }

    var body: some View {
        ZStack {
            contentView
                .overlay(alignment: .center) {
                    if userStore.isLoading {
                        ProgressView()
                            .padding()
                            .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
                    }
                }
            sidebar
        }
        .sheet(
            isPresented: Binding(
                get: { !userStore.isLoading && userStore.current == nil },
                set: { _ in }
            )
        ) {
            LoginPage()
                .interactiveDismissDisabled()
        }
    }
}

#Preview {
    ContentView()
}
