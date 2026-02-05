//
//  SSHomeContent.swift
//  SigSong
//
//  Created by ZigengM1 on 2024/01/27.
//

import Foundation
import SigsongSDK
import SwiftUI
import UIKit

struct SearchItem: Identifiable, Hashable {
    let id: UUID = UUID()
    let title: String
}

@MainActor
class SearchContentViewModel: ObservableObject {
    @Published var searchPlaceHolder: String = ""
    @Published var hotSearchs: [SearchItem] = []
    @Published var historySearchs: [SearchItem] = []
    @Published var searchResults: [ClSongBrief] = []
    @Published var isSearching: Bool = false
    @Published var errorMessage: String?
    @Published var lastQuery: String?
}

extension SearchContentViewModel {
    func firstFeatch() {
        fetchHot()
        fetchHistory()
    }
    func fetchHot() {
        searchPlaceHolder = "搜索预设"
        let tags: [String] = ["一切皆dfdggddfghjmdfghjsxdcfvgbhnxdcfvgbhgdggdgd搜", "8888888888888888888888", "一切皆dfdggdgdggdgd搜", "人声便宜", "一切皆dfdggdgdggdgd搜", "人声", "梦幻进度", "一切皆dfdggdgdggdgd搜", "四川话", "一切皆dfdggdgdggdgd搜", "一切皆搜", "888"]
        let array = NSOrderedSet(array: tags).array as! [String]
        hotSearchs = array.map {
            SearchItem(title: $0)
        }
    }
    func fetchHistory() {
        let tags: [String] = ["一切皆dfdggddfghjmdfghjsxdcfvgbhnxdcfvgbhgdggdgd搜", "8888888888888888888888", "一切皆dfdggdgdggdgd搜", "人声便宜", "一切皆dfdggdgdggdgd搜", "人声", "梦幻进度", "一切皆dfdggdgdggdgd搜", "四川话", "一切皆dfdggdgdggdgd搜", "一切皆搜", "888"]
        let array = NSOrderedSet(array: tags).array as! [String]

        historySearchs = array.map {
            SearchItem(title: $0)
        }
    }

    func recordHistory(keyword: String) {
        let trimmed = keyword.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }

        historySearchs.removeAll { $0.title == trimmed }
        historySearchs.insert(SearchItem(title: trimmed), at: 0)
        if historySearchs.count > 10 {
            historySearchs = Array(historySearchs.prefix(10))
        }
    }
}

struct SongSearchRecommendView: View {
    @ObservedObject var vm: SearchContentViewModel
    var onCilckSearchItem: ((SearchItem) -> Void)?
    var onSelectSong: ((ClSongBrief) -> Void)?

    struct HistoryUICons {
        static var padding: CGFloat { 8 }
        static var fontSize: CGFloat { 17 }
    }

    var hots: some View {
        VStack {
            Text("热门搜索")
            // 定义两列的布局
            let columns: [GridItem] = [
                GridItem(.flexible()),
                GridItem(.flexible())
            ]
            LazyVGrid(columns: columns, spacing: 4) {
                ForEach(vm.hotSearchs, id: \.self) { item in
                    Text("Item \(item.title)")
                        .padding(.vertical, 5)
                        .lineLimit(1)
                        .frame(maxWidth: .infinity)
                        .background(Color.gray)
                        .onTapGesture {
                            onCilckSearchItem?(item)
                        }
                }
            }
            .padding()
        }
    }

    var historys: some View {
        return VStack {
            FlowLayout(items: Array(vm.historySearchs.prefix(10)),
                       viewForTag: { item in
                Text(item.title)
                    .lineLimit(1)
                    .font(.system(size: HistoryUICons.fontSize))
                    .padding(HistoryUICons.padding)
                    .background(Color.gray)
                    .foregroundColor(.white)
                    .cornerRadius(6)
                    .onTapGesture {
                        onCilckSearchItem?(item)
                    }
            }, preferWidthForTag: { item in
                let padding: CGFloat = HistoryUICons.padding * 2  // 两边的 padding
                let font = UIFont.systemFont(ofSize: HistoryUICons.fontSize) // 使用系统默认字体和大小
                let attributes: [NSAttributedString.Key: Any] = [.font: font]
                let size = (item.title as NSString).size(withAttributes: attributes)
                return size.width + padding
            }, preferHeightForTag: UIFont.systemFont(ofSize: HistoryUICons.fontSize).lineHeight)
            .alignmentGuide(.top, computeValue: { _ in
                return 0
            })
            .padding(.horizontal, 10)
        }
    }

    var searchResults: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("搜索结果")
                .font(.headline)
                .padding(.horizontal, 12)

            LazyVStack(spacing: 12) {
                ForEach(vm.searchResults, id: \.id) { brief in
                    Button {
                        onSelectSong?(brief)
                    } label: {
                        VStack(alignment: .leading, spacing: 6) {
                            Text(brief.title)
                                .font(.body)
                                .foregroundStyle(.primary)
                            Text("ID: \(brief.id)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(12)
                        .background(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .fill(Color(.secondarySystemBackground))
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 12)
        }
    }

    var emptyResults: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let query = vm.lastQuery {
                Text("没有找到与 \(query) 相关的歌曲")
            } else {
                Text("没有找到相关的歌曲")
            }
        }
        .font(.callout)
        .foregroundStyle(.secondary)
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 12)
    }

    var searchError: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("搜索失败")
                .font(.headline)
            if let error = vm.errorMessage {
                Text(error)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 12)
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                if let _ = vm.lastQuery {
                    if vm.isSearching {
                        ProgressView("搜索中…")
                            .frame(maxWidth: .infinity, alignment: .center)
                    } else if vm.errorMessage != nil {
                        searchError
                    } else if vm.searchResults.isEmpty {
                        emptyResults
                    } else {
                        searchResults
                    }
                } else {
                    if !vm.hotSearchs.isEmpty {
                        hots
                    }
                    if !vm.historySearchs.isEmpty {
                        historys
                    }
                }
            }
            .padding(.vertical, 16)
        }
    }
}

extension SongSearchRecommendView {
    func onCilckSearchItem(_ action: @escaping ((SearchItem) -> Void)) -> Self {
        var view = self
        view.onCilckSearchItem = action
        return view
    }

    func onSelectSong(_ action: @escaping (ClSongBrief) -> Void) -> Self {
        var view = self
        view.onSelectSong = action
        return view
    }
}

#Preview {
    let vm = SearchContentViewModel()
    vm.firstFeatch()
    return SongSearchRecommendView(vm: vm)
}
