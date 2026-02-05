//
// FollowDragGesture.swift
// swiftuidemo
//
// Created by Zigeng on 2024/1/25.
//

import Foundation
import SwiftUI

struct FollowDragGesture: Gesture {

    let dragGesture: DragGesture = DragGesture()
    var onChangedAction: ((DragGesture.Value) -> Void)?
    var onEndedAction: ((DragGesture.Value) -> Void)?

    var maxLimit: CGFloat?
    var minLimit: CGFloat?
    var offsetValue: Binding<CGFloat>?
    var direction: Axis = .horizontal
    var minEnd: ((_ velocity: CGFloat) -> Void)?
    var maxEnd: ((_ velocity: CGFloat) -> Void)?

    @GestureState private var dragState = DragState.inactive
    enum DragState {
        case inactive
        case dragging(translation: CGSize)
    }

    var body: some Gesture {
        DragGesture()
            .onChanged { value in
                followOnChange(value)
                self.onChangedAction?(value)
            }
            .onEnded { value in
                followOnEnd(value)
                onEndedAction?(value)
            }
    }
}

extension FollowDragGesture {
    func followOnChange(_ value: DragGesture.Value) {
        let rate = abs(value.translation.width / value.translation.height)
        guard (direction == .horizontal ? rate > 2 : rate < 0.5) || abs(value.velocity.height) < 500 else {
            return
        }
        let gestureOffset = direction == .horizontal ? value.translation.width : value.translation.height
        let offset = if let minLimit = minLimit, let maxLimit = maxLimit {
            min(maxLimit, max(minLimit, gestureOffset))
        } else if let minLimit = minLimit {
            max(minLimit, gestureOffset)
        } else if let maxLimit = maxLimit {
            min(maxLimit, gestureOffset)
        } else {
            gestureOffset
        }
        offsetValue?.wrappedValue = offset
    }

    func followOnEnd(_ value: DragGesture.Value) {
        let offset = direction == .horizontal ? value.translation.width : value.translation.height
        let velocity = direction == .horizontal ? value.velocity.width : value.velocity.height
        if let maxEnd = maxEnd, velocity > 700 {
            maxEnd(velocity)
        } else if let minEnd = minEnd, velocity < -700 {
            minEnd(velocity)
        } else if let minEnd = minEnd,
                  let maxLimit = maxLimit,
                  let minLimit = minLimit,
                  offset < (minLimit + (maxLimit - minLimit) / 3) {
            minEnd(velocity)
        } else if let maxEnd = maxEnd,
                  let maxLimit = maxLimit,
                  let minLimit = minLimit,
                  offset < (maxLimit - (maxLimit - minLimit) / 3) {
            maxEnd(velocity)
        } else {
            withAnimation {
                offsetValue?.wrappedValue = 0
            }
        }
    }
}

extension FollowDragGesture {
    func onChanged(_ action: @escaping (DragGesture.Value) -> Void) -> Self {
        var gesture = self
        gesture.onChangedAction = action
        return gesture
    }

    func onEnded(_ action: @escaping (DragGesture.Value) -> Void) -> Self {
        var gesture = self
        gesture.onEndedAction = action
        return gesture
    }

    func bindOffset(_ offsetValue: Binding<CGFloat>) -> Self {
        var gesture = self
        gesture.offsetValue = offsetValue
        return gesture
    }

    func maxLimit(_ limit: CGFloat) -> Self {
        var gesture = self
        gesture.maxLimit = limit
        return gesture
    }

    func minLimit(_ limit: CGFloat) -> Self {
        var gesture = self
        gesture.minLimit = limit
        return gesture
    }

    func maxEnd(_ end: @escaping (_ velocity: CGFloat) -> Void) -> Self {
        var gesture = self
        gesture.maxEnd = end
        return gesture
    }

    func minEnd(_ end: @escaping (_ velocity: CGFloat) -> Void) -> Self {
        var gesture = self
        gesture.minEnd = end
        return gesture
    }
}
