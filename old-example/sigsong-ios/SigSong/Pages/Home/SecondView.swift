//
//  ContentView.swift
//  SigSong
//
//  Created by ZigengM1 on 2023/11/05.
//

import SwiftUI
import MusicKit
import SigsongSDK

struct SecondView: View {
    var body: some View {
        return VStack {
            Text("Second View")
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .navigationBarBackButtonHidden()
        //        .navigationBarTitle("Second View")
        .background(Color.red)
    }
}

#Preview {
    ContentView()
}
