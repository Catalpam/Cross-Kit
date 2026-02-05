//
//  NTAvatar.swift
//  SigSong
//
//  Created by ZigengM1 on 2024/01/27.
//

import Foundation
import SwiftUI
import SDWebImage
import SDWebImageSwiftUI

struct NTAvatar: View {
//    @Binding var key: String
    var url: URL {
        URL(string: "https://s1.4sai.com/src/img/png/8f/8fd48d4c10964ff39a4593f5236ccb25.png?imageView2/2/w/200&e=1735488000&token=1srnZGLKZ0Aqlz6dk7yF4SkiYf4eP-YrEOdM1sob:JgaPcdiUxKfHwpN0mq8l55r6gLM=")!
    }
    var body: some View {
        WebImage(url: url)
            .resizable()
            .frame(maxWidth: 100, maxHeight: 100)
            .aspectRatio(contentMode: .fill) // 保持图片的宽高比
            .clipShape(Circle())
    }
}
