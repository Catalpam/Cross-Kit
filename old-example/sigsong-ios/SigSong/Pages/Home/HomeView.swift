//
//  HomeView.swift
//  SigSong
//
//  Created by ZigengM1 on 2024/01/27.
//

import Foundation
import SwiftUI
import SigsongSDK
import InvokeKit
import SDWebImageSwiftUI

@MainActor
class HomeViewModel: ObservableObject {
    var searchVM = SearchContentViewModel()
    @Published var inputSearchText: String = ""

    @Published private(set) var feed: [ClSongBrief] = []
    @Published private(set) var isLoadingFirstPage = false
    @Published private(set) var isLoadingMore = false
    @Published var errorMessage: String?

    private var lastRecommendId: Int32?
    private var hasMore = true

    init() {
        Task { await loadInitialIfNeeded() }
    }

    func loadInitialIfNeeded() async {
        guard feed.isEmpty else { return }
        await refresh()
    }

    func refresh() async {
        guard !isLoadingFirstPage else { return }
        isLoadingFirstPage = true
        defer { isLoadingFirstPage = false }
        lastRecommendId = nil
        hasMore = true
        feed.removeAll()
        await loadNextPage()
    }

    func loadNextPageIfNeeded(current item: ClSongBrief?) async {
        guard let item else { return }
        let thresholdIndex = feed.index(feed.endIndex, offsetBy: -4, limitedBy: feed.startIndex) ?? feed.startIndex
        if feed.indices.contains(thresholdIndex), feed[thresholdIndex].id == item.id {
            await loadNextPage()
        }
    }

    private func loadNextPage() async {
        guard !isLoadingMore, hasMore else { return }
        isLoadingMore = true
        defer { isLoadingMore = false }

        do {
            let response = try await API.getFeedRecommendSongs(lastRecommendedId: lastRecommendId)
            feed.append(contentsOf: response.songBriefs)
            lastRecommendId = response.recommandId
            hasMore = !response.songBriefs.isEmpty
            errorMessage = nil
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func search(using keyword: String) {
        let trimmed = keyword.trimmingCharacters(in: .whitespacesAndNewlines)
        inputSearchText = trimmed

        guard !trimmed.isEmpty else {
            clearSearch()
            return
        }

        searchVM.lastQuery = trimmed
        Task { await performSearch(keyword: trimmed) }
    }

    func clearSearch() {
        searchVM.lastQuery = nil
        searchVM.searchResults = []
        searchVM.errorMessage = nil
        searchVM.isSearching = false
    }

    private func performSearch(keyword: String) async {
        searchVM.isSearching = true
        searchVM.errorMessage = nil

        let result: Result<[ClSongBrief], Error>
        do {
            let briefs = try await API.searchSong(keyword: keyword)
            result = .success(briefs)
        } catch {
            result = .failure(error)
        }

        defer { searchVM.isSearching = false }

        guard searchVM.lastQuery == keyword else { return }

        switch result {
        case .success(let briefs):
            searchVM.searchResults = briefs
            searchVM.errorMessage = nil
            searchVM.recordHistory(keyword: keyword)
        case .failure(let error):
            searchVM.searchResults = []
            searchVM.errorMessage = error.localizedDescription
        }
    }
}

struct HomeView: View {
    @State private var isInSearchMode = false
    @StateObject var vm = HomeViewModel()
    var onTapAvatar: (() -> Void)?

    @EnvironmentObject private var router: AppRouter

    private let gridColumns = [
        GridItem(.adaptive(minimum: 168), spacing: 16, alignment: .top)
    ]

    var header: some View {
        HStack(spacing: 12) {
            NTAvatar()
                .frame(width: 40, height: 40)
                .onTapGesture(perform: onTapAvatar ?? {})

            NTSearchBar(
                inputSearchText: $vm.inputSearchText,
                isInSearchMode: $isInSearchMode,
                onSubmit: { vm.search(using: $0) },
                onTextChange: { newValue in
                    if newValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                        vm.clearSearch()
                    }
                }
            )
        }
        .padding(.top, 8)
    }

    var searchContent: some View {
        SongSearchRecommendView(vm: vm.searchVM)
            .onCilckSearchItem {
                vm.search(using: $0.title)
            }
            .onSelectSong { brief in
                router.openLyric(id: brief.id)
                isInSearchMode = false
            }
    }

    var feedContent: some View {
        VStack(alignment: .leading, spacing: 18) {
            if let error = vm.errorMessage, vm.feed.isEmpty {
                VStack(spacing: 12) {
                    Text("加载推荐失败")
                        .font(.headline)
                    Text(error)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                    Button("重新加载") {
                        Task { await vm.refresh() }
                    }
                    .buttonStyle(.borderedProminent)
                }
                .frame(maxWidth: .infinity, alignment: .center)
            } else if vm.isLoadingFirstPage && vm.feed.isEmpty {
                ProgressView()
                    .frame(maxWidth: .infinity, alignment: .center)
            } else if vm.feed.isEmpty {
                Text("暂无推荐，请下拉刷新。")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: gridColumns, spacing: 18) {
                    ForEach(vm.feed, id: \.id) { brief in
                        SongPreviewCard(brief: brief)
                            .onTapGesture {
                                router.openLyric(id: brief.id)
                            }
                            .onAppear {
                                Task { await vm.loadNextPageIfNeeded(current: brief) }
                            }
                    }
                }

                if vm.isLoadingMore {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                }
            }
        }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                header

                if isInSearchMode {
                    searchContent
                } else {
                    feedContent
                }
            }
            .padding(.horizontal, 20)
            .padding(.bottom, 140)
        }
        .background(Color(.systemGroupedBackground).ignoresSafeArea())
        .refreshable { await vm.refresh() }
        .task { await vm.loadInitialIfNeeded() }
        .onAppear { vm.searchVM.firstFeatch() }
    }
}

private struct SongPreviewCard: View {
    let brief: ClSongBrief

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            WebImage(url: URL(string: brief.coverImageUrl))
                .resizable()
                .placeholder {
                    ZStack {
                        RoundedRectangle(cornerRadius: 18, style: .continuous)
                            .fill(Color(.secondarySystemBackground))
                        ProgressView()
                    }
                }
                .scaledToFill()
                .frame(height: 160)
                .clipped()
                .clipShape(RoundedRectangle(cornerRadius: 18, style: .continuous))

            Text(brief.title)
                .font(.headline)
                .lineLimit(2)
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 20, style: .continuous)
                .fill(Color(.secondarySystemBackground))
        )
    }
}

extension HomeView {
    func onTapAvatar(_ action: @escaping () -> Void) -> Self {
        var sshome = self
        sshome.onTapAvatar = action
        return sshome
    }
}

#Preview {
    let view = HomeView()
    view.vm.searchVM.firstFeatch()
    return view
}
