//
//  Scroll.swift
//  SigSong
//
//  Created by zigengm3 on 2024/9/15.
//

import Foundation
import SwiftUI
import UIKit
import Combine

struct ScrollOffsetPreferenceKey: PreferenceKey {
    static var defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = nextValue()
    }
}
