import SwiftUI
import UIKit
import SigsongSDK
import InvokeKit

private let surfaceFont = UIFont.systemFont(ofSize: 20)
private let rubyFont = UIFont.systemFont(ofSize: 12)
private let preferredTokenHeight: CGFloat = surfaceFont.lineHeight + rubyFont.lineHeight

private func measureTextWidth(_ text: String, font: UIFont) -> CGFloat {
    guard !text.isEmpty else { return 0 }
    let attributes: [NSAttributedString.Key: Any] = [.font: font]
    return (text as NSString).size(withAttributes: attributes).width
}

private func preferredTokenWidth(for element: ClSongLyricElement) -> CGFloat {
    let surfaceWidth = element.pronouncedString.reduce(CGFloat.zero) { partial, chunk in
        partial + measureTextWidth(chunk.original, font: surfaceFont)
    }

    let rubyWidth = element.pronouncedString.reduce(CGFloat.zero) { partial, chunk in
        guard let pronunciation = chunk.pronunciation else { return partial }
        return partial + measureTextWidth(pronunciation, font: rubyFont)
    }

    let fallback = measureTextWidth(element.surface, font: surfaceFont)
    let width = max(surfaceWidth, rubyWidth, fallback)
    return width
}

struct LyricsView: View {
    @State var lyrics: [ClSongLyric]
    @State private var selectedElementID: Int32?
    @State private var expandedLyricID: Int32?

    var body: some View {
        LazyVStack(alignment: .leading, spacing: 20) {
            ForEach(lyrics, id: \.id) { lyric in
                LyricLineView(
                    lyric: lyric,
                    selectedElementID: $selectedElementID,
                    expandedLyricID: $expandedLyricID
                )
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct LyricLineView: View {
    let lyric: ClSongLyric
    @Binding var selectedElementID: Int32?
    @Binding var expandedLyricID: Int32?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            FlowLayout(items: lyric.elements, viewForTag: { element in
                LyricTokenView(
                    element: element,
                    isSelected: Binding(
                        get: { selectedElementID == element.id },
                        set: { newValue in
                            if newValue {
                                selectedElementID = element.id
                            } else if selectedElementID == element.id {
                                selectedElementID = nil
                            }
                        }
                    )
                )
            }, preferWidthForTag: { element in
                preferredTokenWidth(for: element)
            }, preferHeightForTag: preferredTokenHeight)

            if expandedLyricID == lyric.id {
                Text(lyric.zhCn)
                    .font(.callout)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.vertical, 6)
            }
        }
        .onLongPressGesture {
            withAnimation(.easeInOut) {
                expandedLyricID = expandedLyricID == lyric.id ? nil : lyric.id
            }
        }
    }
}

private struct LyricTokenView: View {
    let element: ClSongLyricElement
    @Binding var isSelected: Bool

    @State private var wordInfos: [ClWordInfo]?
    @State private var isLoading = false
    @State private var loadError: String?

    private var hasRuby: Bool {
        element.pronouncedString.contains { $0.pronunciation != nil }
    }

    var body: some View {
        NTPresentView(
            isShow: $isSelected,
            onWillShow: {
                Task { await fetchWordInfoIfNeeded() }
            },
            content: {
                LyricInfoView(
                    word: element.surface,
                    wordTypeText: element.wordType.localizedDescription,
                    wordInfos: wordInfos,
                    isLoading: isLoading,
                    errorMessage: loadError
                )
            },
            label: {
                VStack(spacing: 2) {
                    if hasRuby {
                        HStack(spacing: 0) {
                            ForEach(element.pronouncedString.indices, id: \.self) { p_index in
                                let text = element.pronouncedString[p_index]
                                let width = measureTextWidth(text: text.original, font: .systemFont(ofSize: 20))
                                //                    let kerning = getDynamicKerning(string: text.pronunciation ?? "", availableWidth: width)
                                ZStack {
                                    Text(text.original)
                                        .font(.title2)
                                        .fixedSize()
                                        .layoutPriority(1)
                                        .overlay {
                                            if let pronunciation = text.pronunciation {
                                                Text(pronunciation)
                                                    .font(.system(size: 12))
                                                    .minimumScaleFactor(0.7) // 设置最小缩放因子
                                                    .lineLimit(1) // 设置最大行数
                                                //.kerning(kerning)  //设置字符间距
                                                    .frame(minWidth: width) // 设置最小宽度
                                                .border(Color.red) // 添加边框以显示最小宽度效果
                                                    .fixedSize()
                                                    .offset(x: 0, y: -18)
                                            }
                                        }
                                }
                            }
                        }
                        .foregroundColor(isSelected ? .primary : element.color)
                        .background(isSelected ? Color.yellow.opacity(0.65) : Color.clear)
                    }
                }
            })
    }

    func measureTextWidth(text: String, font: UIFont) -> CGFloat {
        let attributes = [NSAttributedString.Key.font: font]
        let size = (text as NSString).size(withAttributes: attributes)
        return size.width
    }

    func getDynamicKerning(string: String, availableWidth: CGFloat) -> CGFloat {
        if string.count == 1 {
            return 0
        }
        let metrics = UIFontMetrics.default
        let defaultKerning: CGFloat = 0 // 默认字符间距

        let attributes = [NSAttributedString.Key.font: UIFont.systemFont(ofSize: 12, weight: .regular)]
        let attributedString = NSAttributedString(string: string, attributes: attributes)

        let scaledKerning = metrics.scaledValue(for: defaultKerning)

        let textSize = attributedString.size()
        let textWidth = textSize.width

        if textWidth < availableWidth - 8 {
            let additionalKerning = (availableWidth - 8 - textWidth) / CGFloat(string.count - 1)
            return scaledKerning + additionalKerning
        } else {
            return scaledKerning
        }
    }

    private func fetchWordInfoIfNeeded() async {
        guard wordInfos == nil, !isLoading else { return }
        await MainActor.run {
            isLoading = true
            loadError = nil
        }

        do {
            let infos = try await API.getWordInfo(word: element.surface)
            await MainActor.run {
                wordInfos = infos
                isLoading = false
                if infos.isEmpty {
                    loadError = "暂无相关释义"
                }
            }
        } catch {
            await MainActor.run {
                loadError = error.localizedDescription
                isLoading = false
            }
        }

    }
}

private extension ClSongLyricElement {
    var color: Color {
        switch wordType {
        case .unknown: .gray
        case .noun: .green
        case .verb: .blue
        case .adjective: .orange
        case .adverb: .red
        case .pronoun: .purple
        case .interjection: Color(.magenta)
        case .particle: .indigo
        case .auxiliaryVerb: .cyan
        case .adnominal: .teal
        case .adjectivalNoun: .brown
        case .suffix: .gray
        }
    }
}

private extension WordType {
    var localizedDescription: String {
        switch self {
        case .verb:
            return "动词"
        case .auxiliaryVerb:
            return "助动词"
        case .noun:
            return "名词"
        case .particle:
            return "助词"
        case .adnominal:
            return "连体词"
        case .pronoun:
            return "代词"
        case .adverb:
            return "副词"
        case .adjectivalNoun:
            return "形容动词"
        case .adjective:
            return "形容词"
        case .interjection:
            return "感叹词"
        case .suffix:
            return "接尾词"
        case .unknown:
            return "其他"
        }
    }
}

#Preview {
    ScrollView {
        LyricsView(lyrics: MockData.song.lyrics)
            .padding()
    }
}
