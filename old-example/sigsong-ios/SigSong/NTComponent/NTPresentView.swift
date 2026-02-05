//
//  NTPresentView.swift
//  SigSong
//
//  Created by zigengm3 on 2024/8/3.
//

import Foundation
import SwiftUI

struct NTPresentView<Label, Content>: View where Label: View, Content: View {
    let content: () -> Content
    let label: () -> Label
    let onWillShow: (() -> Void)?

    @Binding var isShow: Bool
    init(isShow: Binding<Bool>,
         onWillShow: (() -> Void)? = nil,
         @ViewBuilder content: @escaping () -> Content,
         @ViewBuilder label: @escaping () -> Label) {
        self.content = content
        self.label = label
        self.onWillShow = onWillShow
        self._isShow = isShow
    }

    var body: some View {
        label()
            .contentShape(Rectangle())
            .onTapGesture {
                onWillShow?()
                isShow = true
            }
            .popover(
            isPresented: $isShow,
            attachmentAnchor: .rect(.bounds),
            arrowEdge: .bottom,
            content: {
                content()
                    .presentationDetents([.fraction(0.2), .medium])
//                    .presentationSizing(.padded)
//                    .presentationCompactAdaptation((.popover))
            }
        )
        .background()
    }
}

struct PopoverView: View {
    @Binding var isPresented: Bool
    let popoverPosition: CGPoint

    var body: some View {
        ZStack {
            Color.black.opacity(0.4)
                .edgesIgnoringSafeArea(.all)
                .onTapGesture {
                    isPresented = false
                }

            VStack {
                Text("Popover Content")
                    .padding()

                Button("Close") {
                    isPresented = false
                }
                .padding()
            }
            .background(Color.white)
            .cornerRadius(10)
            .shadow(radius: 5)
            .position(popoverPosition)
        }
    }
}

//extension NTPresentView {
//    enum HighlightType {
//        case none
//        case bold
//        case background(Color)
//    }
//    func highlight(_ highlight: HighlightType) -> Self {
//        var view = self
//        view.highlightType = highlight
//        return view
//    }
//}
