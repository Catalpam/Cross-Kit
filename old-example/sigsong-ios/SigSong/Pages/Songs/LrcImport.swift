//
//  LrcImport.swift
//  SigSong
//
//  Created by zigengm3 on 2024/8/20.
//

import Foundation
import SwiftUI
import InvokeKit
import SigsongSDK

struct LrcImporter: View {
    @State var showImpoter: Bool = false
//    let importedLrc: (SongInfo) -> Void
    var body: some View {
        Button(action: {
            showImpoter.toggle()
        }, label: {
            Text("Load")
                .padding(8)
                .frame(maxWidth: 500)
        })
        .padding()
        .buttonBorderShape(.capsule)
        .buttonStyle(.borderedProminent)
        .fileImporter(isPresented: $showImpoter, allowedContentTypes: [.init(filenameExtension: "lrc")!, .mp3]) { result in
            switch result {
            case .success(let fileUrl):
                print(fileUrl.path)
                guard fileUrl.startAccessingSecurityScopedResource() else { return }
//                Task {
//                    do {
//                        let song = try API.importLrc(lrcPath: fileUrl.path)
//                        importedLrc(song)
//                        fileUrl.stopAccessingSecurityScopedResource()
//                    } catch {
//                        print(error)
//                    }
//                }
            case .failure(let error):
                print(error.localizedDescription)
            }
        }
    }
}
