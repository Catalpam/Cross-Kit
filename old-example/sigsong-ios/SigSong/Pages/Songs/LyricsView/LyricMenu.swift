import Foundation
import SwiftUI
import SigsongSDK

public struct LyricInfoView: View {
    let word: String
    let wordTypeText: String
    let wordInfos: [ClWordInfo]?
    let isLoading: Bool
    let errorMessage: String?

    public init(word: String, wordTypeText: String, wordInfos: [ClWordInfo]?, isLoading: Bool, errorMessage: String?) {
        self.word = word
        self.wordTypeText = wordTypeText
        self.wordInfos = wordInfos
        self.isLoading = isLoading
        self.errorMessage = errorMessage
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                header

                infoContent
            }
            .padding(.vertical, 16)
            .padding(.horizontal, 20)
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(word)
                .font(.title3)
                .fontWeight(.semibold)

            if !wordTypeText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                Text(wordTypeText)
                    .font(.footnote)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 4)
                    .background(Color.orange.opacity(0.2), in: Capsule())
                    .foregroundStyle(.orange)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private extension LyricInfoView {
    @ViewBuilder
    var infoContent: some View {
        if isLoading {
            ProgressView().frame(maxWidth: .infinity, alignment: .center)
        } else if let errorMessage = errorMessage {
            Text(errorMessage)
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
        } else if let infos = wordInfos, !infos.isEmpty {
            VStack(alignment: .leading, spacing: 16) {
                ForEach(Array(infos.enumerated()), id: \.0) { _, info in
                    VStack(alignment: .leading, spacing: 14) {
                        VStack(alignment: .leading, spacing: 6) {
                            HStack(spacing: 6) {
                                Image(systemName: "speaker.wave.2.fill")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                Text(info.pronounce)
                                    .font(.footnote)
                                    .foregroundStyle(.secondary)
                            }

                            if !info.normalizedForm.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                                Text("原形: \(info.normalizedForm)")
                                    .font(.footnote)
                                    .foregroundStyle(.secondary)
                            }

                            if !info.form_desc.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                                Text("词形: \(info.form_desc)")
                                    .font(.footnote)
                                    .foregroundStyle(.secondary)
                            }

                            if !info.tone.isEmpty {
                                Text("音调: \(info.tone.map(String.init).joined(separator: ", "))")
                                    .font(.footnote)
                                    .foregroundStyle(.secondary)
                            }
                        }

                        if !info.senses.isEmpty {
                            VStack(alignment: .leading, spacing: 12) {
                                ForEach(Array(info.senses.enumerated()), id: \.0) { senseIndex, sense in
                                    VStack(alignment: .leading, spacing: 8) {
                                        if !sense.part.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                                            Text("义项 \(senseIndex + 1) · \(sense.part)")
                                                .font(.subheadline)
                                                .fontWeight(.semibold)
                                        }

                                        ForEach(Array(sense.meanings.enumerated()), id: \.0) { meaningIndex, meaning in
                                            VStack(alignment: .leading, spacing: 6) {
                                                Text("释义 \(meaningIndex + 1): \(meaning.definition)")
                                                    .font(.callout)
                                                    .foregroundStyle(.primary)

                                                if !meaning.examples.isEmpty {
                                                    VStack(alignment: .leading, spacing: 4) {
                                                        ForEach(Array(meaning.examples.enumerated()), id: \.0) { exampleIndex, example in
                                                            VStack(alignment: .leading, spacing: 2) {
                                                                Text("例句 \(exampleIndex + 1): \(example.sentence)")
                                                                    .font(.footnote)
                                                                    .foregroundStyle(.secondary)
                                                                if !example.translation.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                                                                    Text(example.translation)
                                                                        .font(.footnote)
                                                                        .foregroundStyle(.tertiary)
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            .frame(maxWidth: .infinity, alignment: .leading)
                                        }
                                    }
                                }
                            }
                        }
                    }
                    .padding(16)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color(.secondarySystemBackground), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                }
            }
        } else {
            Text("暂无相关释义")
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}
