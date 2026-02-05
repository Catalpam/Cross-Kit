//
//  NTSearchBar.swift
//  SigSong
//
//  Created by ZigengM1 on 2024/01/27.
//

import Foundation
import SwiftUI

struct NTSearchBar: View {
    @Binding var inputSearchText: String
    @Binding var isInSearchMode: Bool
    var onSubmit: ((String) -> Void)? = nil
    var onTextChange: ((String) -> Void)? = nil
    var body: some View {
        TextField("Search", text: $inputSearchText)
            .onTapGesture {
                isInSearchMode = true
            }
            .onSubmit {
                onSubmit?(inputSearchText)
            }
            .onChange(of: inputSearchText) { newValue in
                onTextChange?(newValue)
            }
            .padding(7)
            .padding(.horizontal, 25)
            .background(Color(.systemGray6))
            .cornerRadius(8)
            .overlay(
                HStack {
                    Image(systemName: "magnifyingglass")
                        .foregroundColor(.gray)
                        .frame(minWidth: 0, maxWidth: .infinity, alignment: .leading)
                        .padding(.leading, 8)

                    if !inputSearchText.isEmpty {
                        Button(action: {
                            self.inputSearchText = ""
                        }, label: {
                            Image(systemName: "multiply.circle.fill")
                                .foregroundColor(.gray)
                                .padding(.trailing, 8)
                        })
                    }
                }
            )
    }
}
