//
//  NTFlowLayout.swift
//  SigSong
//
//  Created by ZigengM1 on 2024/01/27.
//

import Foundation
import SwiftUI

struct FlowLayout<Tag: Hashable, Content: View>: View {
    let items: [Tag]
    let viewForTag: (Tag) -> Content
    let preferWidthForTag: (Tag) -> CGFloat
    let preferHeightForTag: CGFloat

    @State private var totalHeight: CGFloat = 0

    var body: some View {
        GeometryReader { geometry in
            VStack {
                HStack {
                    Spacer()
                }
                self.content(in: geometry)
            }
        }
    }

    private func content(in geometry: GeometryProxy) -> some View {
        let containerWidth = geometry.size.width
        var width: CGFloat = 0
        var itemGroups: [[Tag]] = [[]]
        var index = 0
        for item in items {
            let itemWidth = preferWidthForTag(item)
            if width + itemWidth > containerWidth {
                itemGroups.append([item])
                width = itemWidth
                index += 1
            } else {
                itemGroups[index].append(item)
                width += itemWidth
            }
        }
        return VStack(alignment: .leading) {
            ForEach(itemGroups, id: \.self) { itemGroup in
                HStack(spacing: 0) {
                    ForEach(itemGroup, id: \.self) { item in
                        self.viewForTag(item)
//                                                    .padding([.horizontal, .vertical], 4)
                    }
                }
            }
        }
    }
}

#Preview {
    HomeView()
}
