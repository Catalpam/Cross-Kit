////
////  NTcolorStore.swift
////  SigSong
////
////  Created by ZigengM1 on 2024/01/30.
////
//
// import Foundation
// import UIKit
// import SwiftUI
// import Darwin
//
// public var colorStore: NTColorStore = NTColorStore()
//
// extension Color {
//    static func rgb(_ hex: Int) -> Self {
//        let components = (
//            R: Double((hex >> 16) & 0xff) / 255,
//            G: Double((hex >> 08) & 0xff) / 255,
//            B: Double((hex >> 00) & 0xff) / 255
//        )
//        return Color(red: components.R, green: components.G, blue: components.B)
//    }
// }
//
//// struct ThemeColor: Shap {
////    lazy var lm: Color
////    lazy var dm: Color
////    init(_ lm: Int, _ dm: Int) {
////        self.lm = lm
////        self.dm = dm
////    }
////    var color: Color {
////         Color(hex: colorScheme == .dark ? dm : lm)
////    }
//// }
//
// precedencegroup ExponentPrecedentGroup {
//    higherThan: MultiplicationPrecedence
//    associativity: right
// }
//
// infix operator ^ : ExponentPrecedentGroup
//
// let NTColorStoreInstance = NTColorStore()
// public class NTColorStore {
//    var lock = pthread_rwlock_t()
//    init() {
//        Darwin.pthread_rwlock_init(&lock, nil)
//    }
//    deinit {
//        Darwin.pthread_rwlock_destroy(&lock)
//    }
//
//    var _isDark = false
//    public var isDark: Bool {
//        get {
//            Darwin.pthread_rwlock_rdlock(&lock)
//            defer {
//                Darwin.pthread_rwlock_unlock(&lock)
//            }
//            return _isDark
//        }
//        set {
//            pthread_rwlock_wrlock(&lock)
//            defer {
//                Darwin.pthread_rwlock_unlock(&lock)
//            }
//            _isDark = newValue
//        }
//    }
//
//    var store: ColorStore {
//        isDark ? darkStore : lightStore
//    }
//
//    var darkStore: ColorStore
//    var lightStore: ColorStore
// }
//
// func ^ (lhs: Int, rhs: Int) -> Color {
//    // We can omit the return since it's a one-line function
//    @Environment(\.colorScheme) var colorScheme
//    return Color.rgb(colorScheme == .dark ? rhs : lhs)
// }
//
//// fileprivate extension Int {
////    static func
//// }
//// extension Color {
////    static func ~ (lhs: Int, rhs: Int) -> Color {
////    }
//// }
//
// extension Color {
//    struct NT {
////        public static var B50: Color { return rgb(0xF0F4FF) & rgb(0x152340) }
//
////        static var T50: Color { Color.red & Color.blue }
////        static var T50: Color { rgb(0xE2F8F5) & rgb(0x132926) }
//    }
// }
//
//// extension UIColor {
////    public static var Y50: UIColor { return .red & .brown }
//// }
//// struct NTColor {
//// }
//// extension UIColor {
////    static var NT: NTColor
//// }
////
//// extension NTColor {
////    /// light: 0xF0F4FF, dark: 0x152340
////    public static var B50: UIColor { return rgb(0xF0F4FF) & rgb(0x152340) }
////    /// light: 0xE0E9FF, dark: 0x173166
////    public static var B100: UIColor { return rgb(0xE0E9FF) & rgb(0x173166) }
////
////    /// light: 0xC2D4FF, dark: 0x194294
////    public static var B200: UIColor { return rgb(0xC2D4FF) & rgb(0x194294) }
////
////    /// light: 0x94B4FF, dark: 0x2655B6
////    public static var B300: UIColor { return rgb(0x94B4FF) & rgb(0x2655B6) }
////
////    /// light: 0x7AA2FF, dark: 0x275FCE
////    public static var B350: UIColor { return rgb(0x7AA2FF) & rgb(0x275FCE) }
////
////    /// light: 0x5083FB, dark: 0x3370EB
////    public static var B400: UIColor { return rgb(0x5083FB) & rgb(0x3370EB) }
////
////    /// light: 0x336DF4, dark: 0x4C88FF
////    public static var B500: UIColor { return rgb(0x336DF4) & rgb(0x4C88FF) }
////
////    /// light: 0x1456F0, dark: 0x75A4FF
////    public static var B600: UIColor { return rgb(0x1456F0) & rgb(0x75A4FF) }
////
////    /// light: 0x0442D2, dark: 0x8FB4FF
////    public static var B700: UIColor { return rgb(0x0442D2) & rgb(0x8FB4FF) }
////
////    /// light: 0x002F9E, dark: 0xBDD2FF
////    public static var B800: UIColor { return rgb(0x002F9E) & rgb(0xBDD2FF) }
////
////    /// light: 0x002270, dark: 0xE0E9FF
////    public static var B900: UIColor { return rgb(0x002270) & rgb(0xE0E9FF) }
////
//// }
////
// MARK: - Carmine
////
//// extension NTColor {
////    /// light: 0xFEF0F8, dark: 0x3A182B
////    public static var C50: UIColor { return rgb(0xFEF0F8) & rgb(0x3A182B) }
////
////    /// light: 0xFEE2F2, dark: 0x591C3F
////    public static var C100: UIColor { return rgb(0xFEE2F2) & rgb(0x591C3F) }
////
////    /// light: 0xF8C4E1, dark: 0x782B57
////    public static var C200: UIColor { return rgb(0xF8C4E1) & rgb(0x782B57) }
////
////    /// light: 0xF598CC, dark: 0x94386C
////    public static var C300: UIColor { return rgb(0xF598CC) & rgb(0x94386C) }
////
////    /// light: 0xEB78B8, dark: 0xAB417D
////    public static var C350: UIColor { return rgb(0xEB78B8) & rgb(0xAB417D) }
////
////    /// light: 0xDF58A5, dark: 0xC24A8E
////    public static var C400: UIColor { return rgb(0xDF58A5) & rgb(0xC24A8E) }
////
////    /// light: 0xCC398C, dark: 0xDB5EA4
////    public static var C500: UIColor { return rgb(0xCC398C) & rgb(0xDB5EA4) }
////
////    /// light: 0xB82879, dark: 0xED77BA
////    public static var C600: UIColor { return rgb(0xB82879) & rgb(0xED77BA) }
////
////    /// light: 0x9D1562, dark: 0xFC94CF
////    public static var C700: UIColor { return rgb(0x9D1562) & rgb(0xFC94CF) }
////
////    /// light: 0x730744, dark: 0xFFC2E5
////    public static var C800: UIColor { return rgb(0x730744) & rgb(0xFFC2E5) }
////
////    /// light: 0x550C35, dark: 0xFFE0F2
////    public static var C900: UIColor { return rgb(0x550C35) & rgb(0xFFE0F2) }
////
//// }
////
// MARK: - Green
////
//// extension NTColor {
////    /// light: 0xE4FAE1, dark: 0x0E2B0A
////    public static var G50: UIColor { return rgb(0xE4FAE1) & rgb(0x0E2B0A) }
////
////    /// light: 0xD0F5CE, dark: 0x173B12
////    public static var G100: UIColor { return rgb(0xD0F5CE) & rgb(0x173B12) }
////
////    /// light: 0x95E599, dark: 0x21511A
////    public static var G200: UIColor { return rgb(0x95E599) & rgb(0x21511A) }
////
////    /// light: 0x5CD168, dark: 0x296621
////    public static var G300: UIColor { return rgb(0x5CD168) & rgb(0x296621) }
////
////    /// light: 0x35BD4B, dark: 0x2F7526
////    public static var G350: UIColor { return rgb(0x35BD4B) & rgb(0x2F7526) }
////
////    /// light: 0x32A645, dark: 0x35872A
////    public static var G400: UIColor { return rgb(0x32A645) & rgb(0x35872A) }
////
////    /// light: 0x258832, dark: 0x419E34
////    public static var G500: UIColor { return rgb(0x258832) & rgb(0x419E34) }
////
////    /// light: 0x1A7526, dark: 0x51BA43
////    public static var G600: UIColor { return rgb(0x1A7526) & rgb(0x51BA43) }
////
////    /// light: 0x0B6017, dark: 0x69CC5C
////    public static var G700: UIColor { return rgb(0x0B6017) & rgb(0x69CC5C) }
////
////    /// light: 0x04430C, dark: 0x99E490
////    public static var G800: UIColor { return rgb(0x04430C) & rgb(0x99E490) }
////
////    /// light: 0x022C07, dark: 0xCBF5C6
////    public static var G900: UIColor { return rgb(0x022C07) & rgb(0xCBF5C6) }
////
//// }
////
// MARK: - Indigo
////
//// extension NTColor {
////    /// light: 0xF2F3FD, dark: 0x1E204A
////    public static var I50: UIColor { return rgb(0xF2F3FD) & rgb(0x1E204A) }
////
////    /// light: 0xE9EAFB, dark: 0x2A2D69
////    public static var I100: UIColor { return rgb(0xE9EAFB) & rgb(0x2A2D69) }
////
////    /// light: 0xCCCFF9, dark: 0x373D90
////    public static var I200: UIColor { return rgb(0xCCCFF9) & rgb(0x373D90) }
////
////    /// light: 0xABB0F2, dark: 0x474FB8
////    public static var I300: UIColor { return rgb(0xABB0F2) & rgb(0x474FB8) }
////
////    /// light: 0x959BEE, dark: 0x515AD6
////    public static var I350: UIColor { return rgb(0x959BEE) & rgb(0x515AD6) }
////
////    /// light: 0x757DF0, dark: 0x5E68E8
////    public static var I400: UIColor { return rgb(0x757DF0) & rgb(0x5E68E8) }
////
////    /// light: 0x5B65F5, dark: 0x7B83F7
////    public static var I500: UIColor { return rgb(0x5B65F5) & rgb(0x7B83F7) }
////
////    /// light: 0x4752E6, dark: 0x9499F7
////    public static var I600: UIColor { return rgb(0x4752E6) & rgb(0x9499F7) }
////
////    /// light: 0x333DCC, dark: 0xAAAFF8
////    public static var I700: UIColor { return rgb(0x333DCC) & rgb(0xAAAFF8) }
////
////    /// light: 0x1E27A4, dark: 0xCDD0F9
////    public static var I800: UIColor { return rgb(0x1E27A4) & rgb(0xCDD0F9) }
////
////    /// light: 0x151B70, dark: 0xE7E9FE
////    public static var I900: UIColor { return rgb(0x151B70) & rgb(0xE7E9FE) }
////
//// }
////
// MARK: - Lime
////
//// extension NTColor {
////    /// light: 0xF2F8D3, dark: 0x212702
////    public static var L50: UIColor { return rgb(0xF2F8D3) & rgb(0x212702) }
////
////    /// light: 0xE3F0A3, dark: 0x303804
////    public static var L100: UIColor { return rgb(0xE3F0A3) & rgb(0x303804) }
////
////    /// light: 0xC8DD5F, dark: 0x404C06
////    public static var L200: UIColor { return rgb(0xC8DD5F) & rgb(0x404C06) }
////
////    /// light: 0xA2C10B, dark: 0x53610A
////    public static var L300: UIColor { return rgb(0xA2C10B) & rgb(0x53610A) }
////
////    /// light: 0x91AD00, dark: 0x5F700A
////    public static var L350: UIColor { return rgb(0x91AD00) & rgb(0x5F700A) }
////
////    /// light: 0x7B9207, dark: 0x6B7F05
////    public static var L400: UIColor { return rgb(0x7B9207) & rgb(0x6B7F05) }
////
////    /// light: 0x6B7F06, dark: 0x82990A
////    public static var L500: UIColor { return rgb(0x6B7F06) & rgb(0x82990A) }
////
////    /// light: 0x5C6D08, dark: 0x93AF04
////    public static var L600: UIColor { return rgb(0x5C6D08) & rgb(0x93AF04) }
////
////    /// light: 0x4A5804, dark: 0xA6C313
////    public static var L700: UIColor { return rgb(0x4A5804) & rgb(0xA6C313) }
////
////    /// light: 0x333D00, dark: 0xC2E12D
////    public static var L800: UIColor { return rgb(0x333D00) & rgb(0xC2E12D) }
////
////    /// light: 0x262E00, dark: 0xE3F391
////    public static var L900: UIColor { return rgb(0x262E00) & rgb(0xE3F391) }
////
//// }
////
// MARK: - Neutral
////
//// extension NTColor {
////    /// light: 0xffffff, dark: 0x0A0A0A
////    public static var N00: UIColor { return rgb(0xffffff) & rgb(0x0A0A0A) }
////
////    /// light: 0xf5f6f7, dark: 0x1A1A1A
////    public static var N50: UIColor { return rgb(0xf5f6f7) & rgb(0x1A1A1A) }
////
////    /// light: 0xf2f3f5, dark: 0x292929
////    public static var N100: UIColor { return rgb(0xf2f3f5) & rgb(0x292929) }
////
////    /// light: 0xeff0f1, dark: 0x373737
////    public static var N200: UIColor { return rgb(0xeff0f1) & rgb(0x373737) }
////
////    /// light: 0xdee0e3, dark: 0x434343
////    public static var N300: UIColor { return rgb(0xdee0e3) & rgb(0x434343) }
////
////    /// light: 0xd0d3d6, dark: 0x505050
////    public static var N350: UIColor { return rgb(0xd0d3d6) & rgb(0x505050) }
////
////    /// light: 0xbbbfc4, dark: 0x5f5f5f
////    public static var N400: UIColor { return rgb(0xbbbfc4) & rgb(0x5f5f5f) }
////
////    /// light: 0x8f959e, dark: 0x757575
////    public static var N500: UIColor { return rgb(0x8f959e) & rgb(0x757575) }
////
////    /// light: 0x646a73, dark: 0xa6a6a6
////    public static var N600: UIColor { return rgb(0x646a73) & rgb(0xa6a6a6) }
////
////    /// light: 0x51565d, dark: 0xcfcfcf
////    public static var N650: UIColor { return rgb(0x51565d) & rgb(0xcfcfcf) }
////
////    /// light: 0x373c43, dark: 0xe0e0e0
////    public static var N700: UIColor { return rgb(0x373c43) & rgb(0xe0e0e0) }
////
////    /// light: 0x2b2f36, dark: 0xe8e8e8
////    public static var N800: UIColor { return rgb(0x2b2f36) & rgb(0xe8e8e8) }
////
////    /// light: 0x1f2329, dark: 0xebebeb
////    public static var N900: UIColor { return rgb(0x1f2329) & rgb(0xebebeb) }
////
////    /// light: 0x0f1114, dark: 0xf8f8f8
////    public static var N950: UIColor { return rgb(0x0f1114) & rgb(0xf8f8f8) }
////
////    /// light: 0x000000, dark: 0xffffff
////    public static var N1000: UIColor { return rgb(0x000000) & rgb(0xffffff) }
////
//// }
////
// MARK: - Orange
////
//// extension NTColor {
////    /// light: 0xFFF3E5, dark: 0x33210B
////    public static var O50: UIColor { return rgb(0xFFF3E5) & rgb(0x33210B) }
////
////    /// light: 0xFEE7CD, dark: 0x4A2B10
////    public static var O100: UIColor { return rgb(0xFEE7CD) & rgb(0x4A2B10) }
////
////    /// light: 0xFEC48B, dark: 0x683A12
////    public static var O200: UIColor { return rgb(0xFEC48B) & rgb(0x683A12) }
////
////    /// light: 0xFF9D4C, dark: 0x8A4A19
////    public static var O300: UIColor { return rgb(0xFF9D4C) & rgb(0x8A4A19) }
////
////    /// light: 0xFF811A, dark: 0xA15317
////    public static var O350: UIColor { return rgb(0xFF811A) & rgb(0xA15317) }
////
////    /// light: 0xED6D0C, dark: 0xB85E1A
////    public static var O400: UIColor { return rgb(0xED6D0C) & rgb(0xB85E1A) }
////
////    /// light: 0xC25705, dark: 0xDB7018
////    public static var O500: UIColor { return rgb(0xC25705) & rgb(0xDB7018) }
////
////    /// light: 0xA44904, dark: 0xF3871B
////    public static var O600: UIColor { return rgb(0xA44904) & rgb(0xF3871B) }
////
////    /// light: 0x853A05, dark: 0xF89E44
////    public static var O700: UIColor { return rgb(0x853A05) & rgb(0xF89E44) }
////
////    /// light: 0x642B02, dark: 0xFEC88B
////    public static var O800: UIColor { return rgb(0x642B02) & rgb(0xFEC88B) }
////
////    /// light: 0x3B1A02, dark: 0xFEE7CD
////    public static var O900: UIColor { return rgb(0x3B1A02) & rgb(0xFEE7CD) }
////
//// }
////
// MARK: - Purple
////
//// extension NTColor {
////    /// light: 0xF5F0FE, dark: 0x2B194A
////    public static var P50: UIColor { return rgb(0xF5F0FE) & rgb(0x2B194A) }
////
////    /// light: 0xEFE6FE, dark: 0x3F2073
////    public static var P100: UIColor { return rgb(0xEFE6FE) & rgb(0x3F2073) }
////
////    /// light: 0xDCC9FD, dark: 0x5529A3
////    public static var P200: UIColor { return rgb(0xDCC9FD) & rgb(0x5529A3) }
////
////    /// light: 0xC8A9FC, dark: 0x6C39C6
////    public static var P300: UIColor { return rgb(0xC8A9FC) & rgb(0x6C39C6) }
////
////    /// light: 0xB791FA, dark: 0x7C4AD4
////    public static var P350: UIColor { return rgb(0xB791FA) & rgb(0x7C4AD4) }
////
////    /// light: 0x9F6FF1, dark: 0x8C55EC
////    public static var P400: UIColor { return rgb(0x9F6FF1) & rgb(0x8C55EC) }
////
////    /// light: 0x8D55ED, dark: 0xA575FA
////    public static var P500: UIColor { return rgb(0x8D55ED) & rgb(0xA575FA) }
////
////    /// light: 0x7A35F0, dark: 0xB88FFE
////    public static var P600: UIColor { return rgb(0x7A35F0) & rgb(0xB88FFE) }
////
////    /// light: 0x611FD6, dark: 0xC5A3FF
////    public static var P700: UIColor { return rgb(0x611FD6) & rgb(0xC5A3FF) }
////
////    /// light: 0x4811A6, dark: 0xDBC8FD
////    public static var P800: UIColor { return rgb(0x4811A6) & rgb(0xDBC8FD) }
////
////    /// light: 0x2F0080, dark: 0xEFE5FF
////    public static var P900: UIColor { return rgb(0x2F0080) & rgb(0xEFE5FF) }
////
//// }
////
// MARK: - Red
////
//// extension NTColor {
////    /// light: 0xFEF0F0, dark: 0x3D1A19
////    public static var R50: UIColor { return rgb(0xFEF0F0) & rgb(0x3D1A19) }
////
////    /// light: 0xFEE3E2, dark: 0x591F1D
////    public static var R100: UIColor { return rgb(0xFEE3E2) & rgb(0x591F1D) }
////
////    /// light: 0xFDC6C4, dark: 0x7B2524
////    public static var R200: UIColor { return rgb(0xFDC6C4) & rgb(0x7B2524) }
////
////    /// light: 0xF89E9B, dark: 0xA03331
////    public static var R300: UIColor { return rgb(0xF89E9B) & rgb(0xA03331) }
////
////    /// light: 0xFF7570, dark: 0xB33A37
////    public static var R350: UIColor { return rgb(0xFF7570) & rgb(0xB33A37) }
////
////    /// light: 0xF54A45, dark: 0xD14642
////    public static var R400: UIColor { return rgb(0xF54A45) & rgb(0xD14642) }
////
////    /// light: 0xE22E28, dark: 0xF05B56
////    public static var R500: UIColor { return rgb(0xE22E28) & rgb(0xF05B56) }
////
////    /// light: 0xC02A26, dark: 0xF6827E
////    public static var R600: UIColor { return rgb(0xC02A26) & rgb(0xF6827E) }
////
////    /// light: 0xA11C17, dark: 0xF89E9B
////    public static var R700: UIColor { return rgb(0xA11C17) & rgb(0xF89E9B) }
////
////    /// light: 0x741915, dark: 0xFDC6C4
////    public static var R800: UIColor { return rgb(0x741915) & rgb(0xFDC6C4) }
////
////    /// light: 0x590603, dark: 0xFEE3E2
////    public static var R900: UIColor { return rgb(0x590603) & rgb(0xFEE3E2) }
////
//// }
////
// MARK: - Sunflower
////
//// extension NTColor {
////    /// light: 0xFFFFDB, dark: 0x29250A
////    public static var S50: UIColor { return rgb(0xFFFFDB) & rgb(0x29250A) }
////
////    /// light: 0xFFFCA3, dark: 0x38320C
////    public static var S100: UIColor { return rgb(0xFFFCA3) & rgb(0x38320C) }
////
////    /// light: 0xFFF67A, dark: 0x574D01
////    public static var S200: UIColor { return rgb(0xFFF67A) & rgb(0x574D01) }
////
////    /// light: 0xFFF258, dark: 0x7A6A01
////    public static var S300: UIColor { return rgb(0xFFF258) & rgb(0x7A6A01) }
////
////    /// light: 0xFFE928, dark: 0x9C8702
////    public static var S350: UIColor { return rgb(0xFFE928) & rgb(0x9C8702) }
////
////    /// light: 0xE5CE00, dark: 0xC9B218
////    public static var S400: UIColor { return rgb(0xE5CE00) & rgb(0xC9B218) }
////
////    /// light: 0xC2A800, dark: 0xE5CD17
////    public static var S500: UIColor { return rgb(0xC2A800) & rgb(0xE5CD17) }
////
////    /// light: 0x8F7C00, dark: 0xF5DF36
////    public static var S600: UIColor { return rgb(0x8F7C00) & rgb(0xF5DF36) }
////
////    /// light: 0x5C4F00, dark: 0xF7E663
////    public static var S700: UIColor { return rgb(0x5C4F00) & rgb(0xF7E663) }
////
////    /// light: 0x423700, dark: 0xFAED7A
////    public static var S800: UIColor { return rgb(0x423700) & rgb(0xFAED7A) }
////
////    /// light: 0x2C2502, dark: 0xFFF59E
////    public static var S900: UIColor { return rgb(0x2C2502) & rgb(0xFFF59E) }
////
//// }
////
// MARK: - Turquoise
////
//// extension NTColor {
////    /// light: 0xE2F8F5, dark: 0x132926
////    public static var T50: UIColor { return rgb(0xE2F8F5) & rgb(0x132926) }
////
////    /// light: 0xC4F2EC, dark: 0x173B36
////    public static var T100: UIColor { return rgb(0xC4F2EC) & rgb(0x173B36) }
////
////    /// light: 0x6FE8D8, dark: 0x1D4E47
////    public static var T200: UIColor { return rgb(0x6FE8D8) & rgb(0x1D4E47) }
////
////    /// light: 0x33D6C0, dark: 0x25665E
////    public static var T300: UIColor { return rgb(0x33D6C0) & rgb(0x25665E) }
////
////    /// light: 0x2DBEAB, dark: 0x227369
////    public static var T350: UIColor { return rgb(0x2DBEAB) & rgb(0x227369) }
////
////    /// light: 0x10A893, dark: 0x198578
////    public static var T400: UIColor { return rgb(0x10A893) & rgb(0x198578) }
////
////    /// light: 0x0F8575, dark: 0x1FA18F
////    public static var T500: UIColor { return rgb(0x0F8575) & rgb(0x1FA18F) }
////
////    /// light: 0x067062, dark: 0x1AB7A1
////    public static var T600: UIColor { return rgb(0x067062) & rgb(0x1AB7A1) }
////
////    /// light: 0x045D51, dark: 0x17CFB5
////    public static var T700: UIColor { return rgb(0x045D51) & rgb(0x17CFB5) }
////
////    /// light: 0x03443B, dark: 0x65E7D5
////    public static var T800: UIColor { return rgb(0x03443B) & rgb(0x65E7D5) }
////
////    /// light: 0x02312A, dark: 0xB7F7EF
////    public static var T900: UIColor { return rgb(0x02312A) & rgb(0xB7F7EF) }
////
//// }
////
// MARK: - Violet
////
//// extension NTColor {
////    /// light: 0xFCEEFC, dark: 0x3B153B
////    public static var V50: UIColor { return rgb(0xFCEEFC) & rgb(0x3B153B) }
////
////    /// light: 0xF9E2F9, dark: 0x541854
////    public static var V100: UIColor { return rgb(0xF9E2F9) & rgb(0x541854) }
////
////    /// light: 0xF3C4F3, dark: 0x712871
////    public static var V200: UIColor { return rgb(0xF3C4F3) & rgb(0x712871) }
////
////    /// light: 0xE59CE5, dark: 0x8B378B
////    public static var V300: UIColor { return rgb(0xE59CE5) & rgb(0x8B378B) }
////
////    /// light: 0xDE81DE, dark: 0xA43DA4
////    public static var V350: UIColor { return rgb(0xDE81DE) & rgb(0xA43DA4) }
////
////    /// light: 0xCF5ECF, dark: 0xB54AB5
////    public static var V400: UIColor { return rgb(0xCF5ECF) & rgb(0xB54AB5) }
////
////    /// light: 0xBF3DBF, dark: 0xD661D6
////    public static var V500: UIColor { return rgb(0xBF3DBF) & rgb(0xD661D6) }
////
////    /// light: 0xA630A6, dark: 0xE17FE1
////    public static var V600: UIColor { return rgb(0xA630A6) & rgb(0xE17FE1) }
////
////    /// light: 0x872787, dark: 0xE99BE9
////    public static var V700: UIColor { return rgb(0x872787) & rgb(0xE99BE9) }
////
////    /// light: 0x6A116A, dark: 0xF4C3F4
////    public static var V800: UIColor { return rgb(0x6A116A) & rgb(0xF4C3F4) }
////
////    /// light: 0x520052, dark: 0xFCDFFC
////    public static var V900: UIColor { return rgb(0x520052) & rgb(0xFCDFFC) }
////
//// }
////
// MARK: - Wathet
////
//// extension NTColor {
////    /// light: 0xE7F8FE, dark: 0x152830
////    public static var W50: UIColor { return rgb(0xE7F8FE) & rgb(0x152830) }
////
////    /// light: 0xCAEFFC, dark: 0x103647
////    public static var W100: UIColor { return rgb(0xCAEFFC) & rgb(0x103647) }
////
////    /// light: 0x97DCFC, dark: 0x164359
////    public static var W200: UIColor { return rgb(0x97DCFC) & rgb(0x164359) }
////
////    /// light: 0x3EC3F7, dark: 0x135A78
////    public static var W300: UIColor { return rgb(0x3EC3F7) & rgb(0x135A78) }
////
////    /// light: 0x25B0E7, dark: 0x146C91
////    public static var W350: UIColor { return rgb(0x25B0E7) & rgb(0x146C91) }
////
////    /// light: 0x1295CA, dark: 0x1A7FAB
////    public static var W400: UIColor { return rgb(0x1295CA) & rgb(0x1A7FAB) }
////
////    /// light: 0x047FB0, dark: 0x1099CC
////    public static var W500: UIColor { return rgb(0x047FB0) & rgb(0x1099CC) }
////
////    /// light: 0x076A94, dark: 0x25B2E5
////    public static var W600: UIColor { return rgb(0x076A94) & rgb(0x25B2E5) }
////
////    /// light: 0x0F587A, dark: 0x51C3F0
////    public static var W700: UIColor { return rgb(0x0F587A) & rgb(0x51C3F0) }
////
////    /// light: 0x06415C, dark: 0x89DFFE
////    public static var W800: UIColor { return rgb(0x06415C) & rgb(0x89DFFE) }
////
////    /// light: 0x072B3D, dark: 0xC7F0FF
////    public static var W900: UIColor { return rgb(0x072B3D) & rgb(0xC7F0FF) }
////
//// }
////
// MARK: - Yellow
////
//// extension NTColor {
////    /// light: 0xFBF4DF, dark: 0x30250A
////    public static var Y50: UIColor { return rgb(0xFBF4DF) & rgb(0x30250A) }
////    public static var Y50: UIColor { return rgb(0xFBF4DF) & rgb(0x30250A) }
////
////    /// light: 0xFAEDC2, dark: 0x473409
////    public static var Y100: UIColor { return rgb(0xFAEDC2) & rgb(0x473409) }
////
////    /// light: 0xFCDF7E, dark: 0x63470F
////    public static var Y200: UIColor { return rgb(0xFCDF7E) & rgb(0x63470F) }
////
////    /// light: 0xFAD355, dark: 0x8A6419
////    public static var Y300: UIColor { return rgb(0xFAD355) & rgb(0x8A6419) }
////
////    /// light: 0xFFC60A, dark: 0xAD7D15
////    public static var Y350: UIColor { return rgb(0xFFC60A) & rgb(0xAD7D15) }
////
////    /// light: 0xD99904, dark: 0xD49B0B
////    public static var Y400: UIColor { return rgb(0xD99904) & rgb(0xD49B0B) }
////
////    /// light: 0xAD7A03, dark: 0xF0B622
////    public static var Y500: UIColor { return rgb(0xAD7A03) & rgb(0xF0B622) }
////
////    /// light: 0x865B03, dark: 0xFBCB46
////    public static var Y600: UIColor { return rgb(0x865B03) & rgb(0xFBCB46) }
////
////    /// light: 0x6F4A01, dark: 0xFCD456
////    public static var Y700: UIColor { return rgb(0x6F4A01) & rgb(0xFCD456) }
////
////    /// light: 0x573601, dark: 0xFFDE75
////    public static var Y800: UIColor { return rgb(0x573601) & rgb(0xFFDE75) }
////
////    /// light: 0x382201, dark: 0xFFEAA3
////    public static var Y900: UIColor { return rgb(0x382201) & rgb(0xFFEAA3) }
//// }
////
//// static func getToken() -> [NTColor.Name: UIColor] {
////    var store: [NTColor.Name: UIColor] = [:]
////    SwiftLoadable.startOnlyOnce(key: "UniverseDesignColor_NTColor_registToken")
////
////    if UIDevice.current.userInterfaceIdiom == .pad {
////        store[NTColor.Name("bg-base")] = NTColor.N100 & NTColor.rgb(0x171717)
////    } else {
////        store[NTColor.Name("bg-base")] = NTColor.N100 & NTColor.N00
////    }
////    store[NTColor.Name("bg-body")] = NTColor.N00 & NTColor.N50
////    store[NTColor.Name("bg-body-overlay")] = NTColor.N50 & NTColor.N100
////    store[NTColor.Name("bg-content-base")] = NTColor.rgb(0xF8F9FA) & NTColor.rgb(0x121212)
////    store[NTColor.Name("bg-filler")] = NTColor.N200
////    store[NTColor.Name("bg-float")] = NTColor.N00 & NTColor.N100
////    store[NTColor.Name("bg-float-base")] = NTColor.N100 & NTColor.N50
////    store[NTColor.Name("bg-float-overlay")] = NTColor.N50 & NTColor.N200
////    store[NTColor.Name("bg-float-push")] = NTColor.N00.withAlphaComponent(0.8) & NTColor.N100.withAlphaComponent(0.8)
////    store[NTColor.Name("bg-mask")] = NTColor.rgb(0x000000).withAlphaComponent(0.55) & NTColor.rgb(0x000000).withAlphaComponent(0.6)
////    store[NTColor.Name("bg-pricolor")] = NTColor.primaryPri400
////    store[NTColor.Name("bg-sub-navigation")] = NTColor.bgBodyOverlay & NTColor.rgb(0x262626)
////    store[NTColor.Name("bg-text-selection")] = NTColor.B600.withAlphaComponent(0.3)
////    store[NTColor.Name("bg-tips")] = NTColor.N900 & NTColor.N350
////    store[NTColor.Name("fill-active")] = NTColor.primaryFillTransparent02
////    store[NTColor.Name("fill-disabled")] = NTColor.N400
////    store[NTColor.Name("fill-focus")] = NTColor.N900.withAlphaComponent(0.12)
////    store[NTColor.Name("fill-hover")] = NTColor.N900.withAlphaComponent(0.05)
////    store[NTColor.Name("fill-img-mask")] = NTColor.N00.withAlphaComponent(0.0) & NTColor.rgb(0x000000).withAlphaComponent(0.12)
////    store[NTColor.Name("fill-loading-mask")] = NTColor.N00.withAlphaComponent(0.6)
////    store[NTColor.Name("fill-pressed")] = NTColor.N900.withAlphaComponent(0.12)
////    store[NTColor.Name("fill-selected")] = NTColor.primaryFillTransparent01
////    store[NTColor.Name("fill-tag")] = NTColor.N900.withAlphaComponent(0.1)
////    store[NTColor.Name("function-danger-100")] = NTColor.R100
////    store[NTColor.Name("function-danger-200")] = NTColor.R200
////    store[NTColor.Name("function-danger-300")] = NTColor.R300
////    store[NTColor.Name("function-danger-350")] = NTColor.R350
////    store[NTColor.Name("function-danger-400")] = NTColor.R400
////    store[NTColor.Name("function-danger-50")] = NTColor.R50
////    store[NTColor.Name("function-danger-500")] = NTColor.R500
////    store[NTColor.Name("function-danger-600")] = NTColor.R600
////    store[NTColor.Name("function-danger-700")] = NTColor.R700
////    store[NTColor.Name("function-danger-800")] = NTColor.R800
////    store[NTColor.Name("function-danger-900")] = NTColor.R900
////    store[NTColor.Name("function-danger-on-danger-fill")] = NTColor.staticWhite
////    store[NTColor.Name("function-danger-content-default")] = NTColor.functionDanger400 & NTColor.functionDanger500
////    store[NTColor.Name("function-danger-content-hover")] = NTColor.functionDanger350 & NTColor.functionDanger400
////    store[NTColor.Name("function-danger-content-pressed")] = NTColor.functionDanger500 & NTColor.functionDanger600
////    store[NTColor.Name("function-danger-content-loading")] = NTColor.functionDanger300
////    store[NTColor.Name("function-danger-fill-default")] = NTColor.functionDanger400
////    store[NTColor.Name("function-danger-fill-hover")] = NTColor.functionDanger350
////    store[NTColor.Name("function-danger-fill-pressed")] = NTColor.functionDanger500
////    store[NTColor.Name("function-danger-fill-loading")] = NTColor.functionDanger300
////    store[NTColor.Name("function-danger-fill-solid-01")] = NTColor.functionDanger50
////    store[NTColor.Name("function-danger-fill-solid-02")] = NTColor.functionDanger100
////    store[NTColor.Name("function-danger-fill-solid-03")] = NTColor.functionDanger200
////    store[NTColor.Name("function-danger-fill-transparent-01")] = NTColor.functionDanger500.withAlphaComponent(0.1) & NTColor.functionDanger500.withAlphaComponent(0.15)
////    store[NTColor.Name("function-danger-fill-transparent-02")] = NTColor.functionDanger500.withAlphaComponent(0.15) & NTColor.functionDanger500.withAlphaComponent(0.2)
////    store[NTColor.Name("function-danger-fill-transparent-03")] = NTColor.functionDanger500.withAlphaComponent(0.2) & NTColor.functionDanger500.withAlphaComponent(0.3)
////    store[NTColor.Name("text-caption")] = NTColor.N600
////    store[NTColor.Name("function-info-100")] = NTColor.B100
////    store[NTColor.Name("function-info-200")] = NTColor.B200
////    store[NTColor.Name("function-info-300")] = NTColor.B300
////    store[NTColor.Name("function-info-350")] = NTColor.B350
////    store[NTColor.Name("function-info-400")] = NTColor.B400
////    store[NTColor.Name("function-info-50")] = NTColor.B50
////    store[NTColor.Name("function-info-500")] = NTColor.B500
////    store[NTColor.Name("function-info-600")] = NTColor.B600
////    store[NTColor.Name("function-info-700")] = NTColor.B700
////    store[NTColor.Name("function-info-800")] = NTColor.B800
////    store[NTColor.Name("function-info-900")] = NTColor.B900
////    store[NTColor.Name("function-info-on-info-fill")] = NTColor.staticWhite
////    store[NTColor.Name("function-info-content-default")] = NTColor.functionInfo600 & NTColor.functionInfo500
////    store[NTColor.Name("function-info-content-hover")] = NTColor.functionInfo500 & NTColor.functionInfo400
////    store[NTColor.Name("function-info-content-pressed")] = NTColor.functionInfo700 & NTColor.functionInfo600
////    store[NTColor.Name("function-info-content-loading")] = NTColor.functionInfo300
////    store[NTColor.Name("function-info-fill-default")] = NTColor.functionInfo600 & NTColor.functionInfo400
////    store[NTColor.Name("function-info-fill-hover")] = NTColor.functionInfo500 & NTColor.functionInfo350
////    store[NTColor.Name("function-info-fill-pressed")] = NTColor.functionInfo700 & NTColor.functionInfo500
////    store[NTColor.Name("function-info-fill-loading")] = NTColor.functionInfo300
////    store[NTColor.Name("function-info-fill-solid-01")] = NTColor.functionInfo50
////    store[NTColor.Name("function-info-fill-solid-02")] = NTColor.functionInfo100
////    store[NTColor.Name("function-info-fill-solid-03")] = NTColor.functionInfo200
////    store[NTColor.Name("function-info-fill-transparent-01")] = NTColor.functionInfo600.withAlphaComponent(0.1) & NTColor.functionInfo500.withAlphaComponent(0.15)
////    store[NTColor.Name("function-info-fill-transparent-02")] = NTColor.functionInfo600.withAlphaComponent(0.15) & NTColor.functionInfo500.withAlphaComponent(0.2)
////    store[NTColor.Name("function-info-fill-transparent-03")] = NTColor.functionInfo600.withAlphaComponent(0.2) & NTColor.functionInfo500.withAlphaComponent(0.3)
////    store[NTColor.Name("function-success-100")] = NTColor.G100
////    store[NTColor.Name("function-success-200")] = NTColor.G200
////    store[NTColor.Name("function-success-300")] = NTColor.G300
////    store[NTColor.Name("function-success-350")] = NTColor.G350
////    store[NTColor.Name("function-success-400")] = NTColor.G400
////    store[NTColor.Name("function-success-50")] = NTColor.G50
////    store[NTColor.Name("function-success-500")] = NTColor.G500
////    store[NTColor.Name("function-success-600")] = NTColor.G600
////    store[NTColor.Name("function-success-700")] = NTColor.G700
////    store[NTColor.Name("function-success-800")] = NTColor.G800
////    store[NTColor.Name("function-success-900")] = NTColor.G900
////    store[NTColor.Name("function-success-on-success-fill")] = NTColor.staticWhite
////    store[NTColor.Name("function-success-content-default")] = NTColor.functionSuccess400 & NTColor.functionSuccess600
////    store[NTColor.Name("function-success-content-hover")] = NTColor.functionSuccess350 & NTColor.functionSuccess500
////    store[NTColor.Name("function-success-content-pressed")] = NTColor.functionSuccess500 & NTColor.functionSuccess700
////    store[NTColor.Name("function-success-content-loading")] = NTColor.functionSuccess300
////    store[NTColor.Name("function-success-fill-default")] = NTColor.functionSuccess350 & NTColor.functionSuccess500
////    store[NTColor.Name("function-success-fill-hover")] = NTColor.functionSuccess300 & NTColor.functionSuccess400
////    store[NTColor.Name("function-success-fill-pressed")] = NTColor.functionSuccess400 & NTColor.functionSuccess600
////    store[NTColor.Name("function-success-fill-loading")] = NTColor.functionSuccess300
////    store[NTColor.Name("function-success-fill-solid-01")] = NTColor.functionSuccess50
////    store[NTColor.Name("function-success-fill-solid-02")] = NTColor.functionSuccess100
////    store[NTColor.Name("function-success-fill-solid-03")] = NTColor.functionSuccess200
////    store[NTColor.Name("function-success-fill-transparent-01")] = NTColor.functionSuccess350.withAlphaComponent(0.1) & NTColor.functionSuccess500.withAlphaComponent(0.15)
////    store[NTColor.Name("function-success-fill-transparent-02")] = NTColor.functionSuccess350.withAlphaComponent(0.15) & NTColor.functionSuccess500.withAlphaComponent(0.2)
////    store[NTColor.Name("function-success-fill-transparent-03")] = NTColor.functionSuccess350.withAlphaComponent(0.2) & NTColor.functionSuccess500.withAlphaComponent(0.3)
////    store[NTColor.Name("function-warning-100")] = NTColor.O100
////    store[NTColor.Name("function-warning-200")] = NTColor.O200
////    store[NTColor.Name("function-warning-300")] = NTColor.O300
////    store[NTColor.Name("function-warning-350")] = NTColor.O350
////    store[NTColor.Name("function-warning-400")] = NTColor.O400
////    store[NTColor.Name("function-warning-50")] = NTColor.O50
////    store[NTColor.Name("function-warning-500")] = NTColor.O500
////    store[NTColor.Name("function-warning-600")] = NTColor.O600
////    store[NTColor.Name("function-warning-700")] = NTColor.O700
////    store[NTColor.Name("function-warning-800")] = NTColor.O800
////    store[NTColor.Name("function-warning-900")] = NTColor.O900
////    store[NTColor.Name("icon-disabled")] = NTColor.N400
////    store[NTColor.Name("icon-n1")] = NTColor.N800
////    store[NTColor.Name("icon-n2")] = NTColor.N600
////    store[NTColor.Name("icon-n3")] = NTColor.N500
////    store[NTColor.Name("line-border-card")] = NTColor.N300 & NTColor.N900.withAlphaComponent(0.15)
////    store[NTColor.Name("line-border-component")] = NTColor.N350 & NTColor.N900.withAlphaComponent(0.2)
////    store[NTColor.Name("line-divider-default")] = NTColor.N900.withAlphaComponent(0.15) & NTColor.N650.withAlphaComponent(0.15)
////    store[NTColor.Name("line-divider-module")] = NTColor.N900.withAlphaComponent(0.15) & NTColor.staticBlack
////    store[NTColor.Name("primary-pri-100")] = NTColor.B100
////    store[NTColor.Name("primary-pri-200")] = NTColor.B200
////    store[NTColor.Name("primary-pri-300")] = NTColor.B300
////    store[NTColor.Name("primary-pri-350")] = NTColor.B350
////    store[NTColor.Name("primary-pri-400")] = NTColor.B400
////    store[NTColor.Name("primary-pri-50")] = NTColor.B50
////    store[NTColor.Name("primary-pri-500")] = NTColor.B500
////    store[NTColor.Name("primary-pri-600")] = NTColor.B600
////    store[NTColor.Name("primary-pri-700")] = NTColor.B700
////    store[NTColor.Name("primary-pri-800")] = NTColor.B800
////    store[NTColor.Name("primary-pri-900")] = NTColor.B900
////    store[NTColor.Name("shadow-default")] = NTColor.N900 & NTColor.rgb(0x000000)
////    store[NTColor.Name("shadow-default-lg")] = NTColor.N900.withAlphaComponent(0.08) & NTColor.rgb(0x000000).withAlphaComponent(0.24)
////    store[NTColor.Name("shadow-default-md")] = NTColor.N900.withAlphaComponent(0.1) & NTColor.rgb(0x000000).withAlphaComponent(0.28)
////    store[NTColor.Name("shadow-default-sm")] = NTColor.N900.withAlphaComponent(0.12) & NTColor.rgb(0x000000).withAlphaComponent(0.32)
////    store[NTColor.Name("shadow-pri")] = NTColor.primaryPri600 & NTColor.rgb(0x245BDB)
////    store[NTColor.Name("shadow-pri-lg")] = NTColor.primaryPri500.withAlphaComponent(0.24) & NTColor.primaryPri200.withAlphaComponent(0.48)
////    store[NTColor.Name("shadow-pri-md")] = NTColor.primaryPri600.withAlphaComponent(0.24) & NTColor.primaryPri200.withAlphaComponent(0.48)
////    store[NTColor.Name("static-black")] = NTColor.rgb(0x000000)
////    store[NTColor.Name("static-white")] = NTColor.rgb(0xFFFFFF)
////    store[NTColor.Name("static-white-hover")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.1)
////    store[NTColor.Name("static-white-pressed")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.2)
////    store[NTColor.Name("text-disabled")] = NTColor.N400
////    store[NTColor.Name("text-link-disabled")] = NTColor.N400
////    store[NTColor.Name("text-link-hover")] = NTColor.B500 & NTColor.B400
////    store[NTColor.Name("text-link-loading")] = NTColor.B300
////    store[NTColor.Name("text-link-normal")] = NTColor.B600 & NTColor.B500
////    store[NTColor.Name("text-link-pressed")] = NTColor.B700 & NTColor.B600
////    store[NTColor.Name("text-placeholder")] = NTColor.N500
////    store[NTColor.Name("text-title")] = NTColor.N900
////    store[NTColor.Name("N00-10")] = NTColor.N00.withAlphaComponent(0.1)
////    store[NTColor.Name("N00-15")] = NTColor.N00.withAlphaComponent(0.15)
////    store[NTColor.Name("N00-20")] = NTColor.N00.withAlphaComponent(0.2)
////    store[NTColor.Name("N00-30")] = NTColor.N00.withAlphaComponent(0.3)
////    store[NTColor.Name("N00-40")] = NTColor.N00.withAlphaComponent(0.4)
////    store[NTColor.Name("N00-5")] = NTColor.N00.withAlphaComponent(0.05)
////    store[NTColor.Name("N00-50")] = NTColor.N00.withAlphaComponent(0.5)
////    store[NTColor.Name("N00-60")] = NTColor.N00.withAlphaComponent(0.6)
////    store[NTColor.Name("N00-70")] = NTColor.N00.withAlphaComponent(0.7)
////    store[NTColor.Name("N00-80")] = NTColor.N00.withAlphaComponent(0.8)
////    store[NTColor.Name("N00-90")] = NTColor.N00.withAlphaComponent(0.9)
////    store[NTColor.Name("N900-10")] = NTColor.N900.withAlphaComponent(0.1)
////    store[NTColor.Name("N900-15")] = NTColor.N900.withAlphaComponent(0.15)
////    store[NTColor.Name("N900-20")] = NTColor.N900.withAlphaComponent(0.2)
////    store[NTColor.Name("N900-30")] = NTColor.N900.withAlphaComponent(0.3)
////    store[NTColor.Name("N900-40")] = NTColor.N900.withAlphaComponent(0.4)
////    store[NTColor.Name("N900-5")] = NTColor.N900.withAlphaComponent(0.05)
////    store[NTColor.Name("N900-50")] = NTColor.N900.withAlphaComponent(0.5)
////    store[NTColor.Name("N900-60")] = NTColor.N900.withAlphaComponent(0.6)
////    store[NTColor.Name("N900-70")] = NTColor.N900.withAlphaComponent(0.7)
////    store[NTColor.Name("N900-80")] = NTColor.N900.withAlphaComponent(0.8)
////    store[NTColor.Name("N900-90")] = NTColor.N900.withAlphaComponent(0.9)
////    store[NTColor.Name("static-black-10")] = NTColor.rgb(0x000000).withAlphaComponent(0.1)
////    store[NTColor.Name("static-black-15")] = NTColor.rgb(0x000000).withAlphaComponent(0.15)
////    store[NTColor.Name("static-black-20")] = NTColor.rgb(0x000000).withAlphaComponent(0.2)
////    store[NTColor.Name("static-black-30")] = NTColor.rgb(0x000000).withAlphaComponent(0.3)
////    store[NTColor.Name("static-black-40")] = NTColor.rgb(0x000000).withAlphaComponent(0.4)
////    store[NTColor.Name("static-black-5")] = NTColor.rgb(0x000000).withAlphaComponent(0.05)
////    store[NTColor.Name("static-black-50")] = NTColor.rgb(0x000000).withAlphaComponent(0.5)
////    store[NTColor.Name("static-black-60")] = NTColor.rgb(0x000000).withAlphaComponent(0.6)
////    store[NTColor.Name("static-black-70")] = NTColor.rgb(0x000000).withAlphaComponent(0.7)
////    store[NTColor.Name("static-black-80")] = NTColor.rgb(0x000000).withAlphaComponent(0.8)
////    store[NTColor.Name("static-black-90")] = NTColor.rgb(0x000000).withAlphaComponent(0.9)
////    store[NTColor.Name("static-white-10")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.1)
////    store[NTColor.Name("static-white-15")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.15)
////    store[NTColor.Name("static-white-20")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.2)
////    store[NTColor.Name("static-white-30")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.3)
////    store[NTColor.Name("static-white-40")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.4)
////    store[NTColor.Name("static-white-5")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.05)
////    store[NTColor.Name("static-white-50")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.5)
////    store[NTColor.Name("static-white-60")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.6)
////    store[NTColor.Name("static-white-70")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.7)
////    store[NTColor.Name("static-white-80")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.8)
////    store[NTColor.Name("static-white-90")] = NTColor.rgb(0xFFFFFF).withAlphaComponent(0.9)
////    store[NTColor.Name("primary-on-primary-fill")] = NTColor.staticWhite
////    store[NTColor.Name("primary-content-default")] = NTColor.primaryPri600 & NTColor.primaryPri500
////    store[NTColor.Name("primary-content-hover")] = NTColor.primaryPri500 & NTColor.primaryPri400
////    store[NTColor.Name("primary-content-pressed")] = NTColor.primaryPri700 & NTColor.primaryPri600
////    store[NTColor.Name("primary-content-loading")] = NTColor.primaryPri300
////    store[NTColor.Name("colorful-blue")] = NTColor.B500
////    store[NTColor.Name("colorful-red")] = NTColor.R400 & NTColor.R500
////    store[NTColor.Name("colorful-orange")] = NTColor.O350 & NTColor.O600
////    store[NTColor.Name("colorful-yellow")] = NTColor.Y350 & NTColor.Y500
////    store[NTColor.Name("colorful-green")] = NTColor.G350 & NTColor.G500
////    store[NTColor.Name("colorful-wathet")] = NTColor.W350 & NTColor.W500
////    store[NTColor.Name("colorful-indigo")] = NTColor.I500 & NTColor.I400
////    store[NTColor.Name("colorful-purple")] = NTColor.P500 & NTColor.P400
////    store[NTColor.Name("colorful-violet")] = NTColor.V500 & NTColor.V400
////    store[NTColor.Name("colorful-carmine")] = NTColor.C400
////    store[NTColor.Name("colorful-sunflower")] = NTColor.S350 & NTColor.S500
////    store[NTColor.Name("colorful-turquoise")] = NTColor.T350 & NTColor.T500
////    store[NTColor.Name("colorful-lime")] = NTColor.L350 & NTColor.L600
////    store[NTColor.Name("colorful-neutral")] = NTColor.N500
////    store[NTColor.Name("primary-fill-default")] = NTColor.primaryPri600 & NTColor.primaryPri400
////    store[NTColor.Name("primary-fill-hover")] = NTColor.primaryPri500 & NTColor.primaryPri350
////    store[NTColor.Name("primary-fill-pressed")] = NTColor.primaryPri700 & NTColor.primaryPri500
////    store[NTColor.Name("primary-fill-loading")] = NTColor.primaryPri300
////    store[NTColor.Name("primary-fill-solid-01")] = NTColor.primaryPri50
////    store[NTColor.Name("primary-fill-solid-02")] = NTColor.primaryPri100
////    store[NTColor.Name("primary-fill-solid-03")] = NTColor.primaryPri200
////    store[NTColor.Name("primary-fill-transparent-01")] = NTColor.primaryPri600.withAlphaComponent(0.1) & NTColor.primaryPri500.withAlphaComponent(0.15)
////    store[NTColor.Name("primary-fill-transparent-02")] = NTColor.primaryPri600.withAlphaComponent(0.15) & NTColor.primaryPri500.withAlphaComponent(0.2)
////    store[NTColor.Name("primary-fill-transparent-03")] = NTColor.primaryPri600.withAlphaComponent(0.2) & NTColor.primaryPri500.withAlphaComponent(0.3)
////    store[NTColor.Name("function-warning-on-warning-fill")] = NTColor.staticWhite
////    store[NTColor.Name("function-warning-content-default")] = NTColor.functionWarning400 & NTColor.functionWarning600
////    store[NTColor.Name("function-warning-content-hover")] = NTColor.functionWarning350 & NTColor.functionWarning500
////    store[NTColor.Name("function-warning-content-pressed")] = NTColor.functionWarning500 & NTColor.functionWarning700
////    store[NTColor.Name("function-warning-content-loading")] = NTColor.functionWarning300
////    store[NTColor.Name("function-warning-fill-default")] = NTColor.functionWarning350 & NTColor.functionWarning600
////    store[NTColor.Name("function-warning-fill-hover")] = NTColor.functionWarning300 & NTColor.functionWarning500
////    store[NTColor.Name("function-warning-fill-pressed")] = NTColor.functionWarning400 & NTColor.functionWarning700
////    store[NTColor.Name("function-warning-fill-loading")] = NTColor.functionWarning300
////    store[NTColor.Name("function-warning-fill-solid-01")] = NTColor.functionWarning50
////    store[NTColor.Name("function-warning-fill-solid-02")] = NTColor.functionWarning100
////    store[NTColor.Name("function-warning-fill-solid-03")] = NTColor.functionWarning200
////    store[NTColor.Name("function-warning-fill-transparent-01")] = NTColor.functionWarning400.withAlphaComponent(0.1) & NTColor.functionWarning600.withAlphaComponent(0.15)
////    store[NTColor.Name("function-warning-fill-transparent-02")] = NTColor.functionWarning400.withAlphaComponent(0.15) & NTColor.functionWarning600.withAlphaComponent(0.2)
////    store[NTColor.Name("function-warning-fill-transparent-03")] = NTColor.functionWarning400.withAlphaComponent(0.2) & NTColor.functionWarning600.withAlphaComponent(0.3)
////    store[NTColor.Name("Vnext-bg-sub-navigation")] = NTColor.rgb(0xF5F5F5).withAlphaComponent(0.7) & NTColor.rgb(0x1E1E1E).withAlphaComponent(0.5)
////    store[NTColor.Name("Vnext-bg-thirdly-navigation")] = NTColor.rgb(0xFCFCFC).withAlphaComponent(0.9) & NTColor.rgb(0x1A1A1A).withAlphaComponent(0.7)
////    store[NTColor.Name("Vnext-bg-content-header")] = NTColor.rgb(0xFCFCFC) & NTColor.rgb(0x222222)
////    store[NTColor.Name("illustration-blue-a")] = NTColor.B900 & NTColor.N700
////    store[NTColor.Name("AI-cursor")] = NTColor.I600 & NTColor.I400
////    store[NTColor.Name("illustration-blue-b")] = NTColor.B900 & NTColor.staticWhite.withAlphaComponent(0.4)
////    store[NTColor.Name("illustration-blue-c")] = NTColor.B900 & NTColor.N400
////    store[NTColor.Name("illustration-blue-d")] = NTColor.B900 & NTColor.B300
////    store[NTColor.Name("illustration-blue-e")] = NTColor.B500
////    store[NTColor.Name("illustration-neutral-a")] = NTColor.N00 & NTColor.N900
////    store[NTColor.Name("illustration-neutral-b")] = NTColor.N00 & NTColor.N200
////    store[NTColor.Name("illustration-neutral-c")] = NTColor.N400.withAlphaComponent(0.45) & NTColor.staticWhite.withAlphaComponent(0.3)
////    store[NTColor.Name("illustration-neutral-d")] = NTColor.N500 & NTColor.staticWhite.withAlphaComponent(0.4)
////    store[NTColor.Name("illustration-orange")] = NTColor.O350 & NTColor.O600
////    store[NTColor.Name("illustration-red")] = NTColor.R400 & NTColor.R500
////    store[NTColor.Name("illustration-static-black")] = NTColor.staticBlack
////    store[NTColor.Name("illustration-static-white")] = NTColor.staticWhite
////    store[NTColor.Name("illustration-turquoise")] = NTColor.T300 & NTColor.T700
////    store[NTColor.Name("illustration-wathet")] = NTColor.W300 & NTColor.W700
////    store[NTColor.Name("illustration-yellow")] = NTColor.Y350 & NTColor.Y500
////    store[NTColor.Name("NTtoken-block-view-inline-bg-nopermission")] = NTColor.N900.withAlphaComponent(0.1)
////    store[NTColor.Name("NTtoken-block-view-inline-bg-other")] = NTColor.primaryPri300.withAlphaComponent(0.18) & NTColor.primaryPri300.withAlphaComponent(0.35)
////    store[NTColor.Name("NTtoken-block-view-inline-bg-self")] = NTColor.primaryPri600
////    store[NTColor.Name("NTtoken-btn-ghost-bg")] = NTColor.N00.withAlphaComponent(0.0)
////    store[NTColor.Name("NTtoken-btn-pri-text-disabled")] = NTColor.staticWhite & NTColor.N900.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-btn-se-bg-danger-focus")] = NTColor.functionDangerFillSolid02 & NTColor.functionDangerFillTransparent02
////    store[NTColor.Name("NTtoken-btn-se-bg-danger-hover")] = NTColor.functionDangerFillSolid02 & NTColor.functionDangerFillTransparent02
////    store[NTColor.Name("NTtoken-btn-se-bg-danger-pressed")] = NTColor.functionDangerFillSolid03 & NTColor.functionDangerFillTransparent03
////    store[NTColor.Name("NTtoken-btn-se-bg-neutral-focus")] = NTColor.N100 & NTColor.N900.withAlphaComponent(0.1)
////    store[NTColor.Name("NTtoken-btn-se-bg-neutral-hover")] = NTColor.N100 & NTColor.N900.withAlphaComponent(0.1)
////    store[NTColor.Name("NTtoken-btn-se-bg-neutral-pressed")] = NTColor.N200 & NTColor.N900.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-btn-se-bg-pri-focus")] = NTColor.primaryFillSolid02 & NTColor.primaryFillTransparent02
////    store[NTColor.Name("NTtoken-btn-se-bg-pri-hover")] = NTColor.primaryFillSolid02 & NTColor.primaryFillTransparent02
////    store[NTColor.Name("NTtoken-btn-se-bg-pri-pressed")] = NTColor.primaryFillSolid03 & NTColor.primaryFillTransparent03
////    store[NTColor.Name("NTtoken-btn-text-bg-danger-focus")] = NTColor.functionDangerFillTransparent01 & NTColor.functionDangerFillTransparent02
////    store[NTColor.Name("NTtoken-btn-text-bg-danger-hover")] = NTColor.functionDangerFillTransparent01 & NTColor.functionDangerFillTransparent02
////    store[NTColor.Name("NTtoken-btn-text-bg-danger-pressed")] = NTColor.functionDangerFillTransparent03
////    store[NTColor.Name("NTtoken-btn-text-bg-neutral-focus")] = NTColor.N900.withAlphaComponent(0.1)
////    store[NTColor.Name("NTtoken-btn-text-bg-neutral-hover")] = NTColor.N900.withAlphaComponent(0.1)
////    store[NTColor.Name("NTtoken-btn-text-bg-neutral-pressed")] = NTColor.N900.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-btn-text-bg-pri-focus")] = NTColor.primaryFillTransparent01 & NTColor.primaryFillTransparent02
////    store[NTColor.Name("NTtoken-btn-text-bg-pri-hover")] = NTColor.primaryFillTransparent01 & NTColor.primaryFillTransparent02
////    store[NTColor.Name("NTtoken-btn-text-bg-pri-pressed")] = NTColor.primaryFillTransparent03
////    store[NTColor.Name("NTtoken-btn-selected-bg-danger-hover")] = NTColor.functionDangerFillSolid02 & NTColor.functionDangerFillTransparent03
////    store[NTColor.Name("NTtoken-btn-selected-bg-danger-normal")] = NTColor.functionDangerFillSolid01 & NTColor.functionDangerFillTransparent01
////    store[NTColor.Name("NTtoken-btn-selected-bg-danger-press")] = NTColor.functionDangerFillSolid03 & NTColor.functionDangerFillTransparent03
////    store[NTColor.Name("NTtoken-btn-selected-bg-neutral-hover")] = NTColor.N900.withAlphaComponent(0.1) & NTColor.N900.withAlphaComponent(0.12)
////    store[NTColor.Name("NTtoken-btn-selected-bg-neutral-normal")] = NTColor.N900.withAlphaComponent(0.06) & NTColor.N900.withAlphaComponent(0.08)
////    store[NTColor.Name("NTtoken-btn-selected-bg-neutral-press")] = NTColor.N900.withAlphaComponent(0.14) & NTColor.N900.withAlphaComponent(0.16)
////    store[NTColor.Name("NTtoken-btn-selected-bg-success-hover")] = NTColor.functionSuccessFillSolid02 & NTColor.functionSuccessFillTransparent02
////    store[NTColor.Name("NTtoken-btn-selected-bg-success-normal")] = NTColor.functionSuccessFillSolid01 & NTColor.functionSuccessFillTransparent01
////    store[NTColor.Name("NTtoken-btn-selected-bg-success-press")] = NTColor.functionSuccessFillSolid03 & NTColor.functionSuccessFillTransparent03
////    store[NTColor.Name("NTtoken-btn-selected-border-neutral")] = NTColor.N1000.withAlphaComponent(0.3) & NTColor.N1000.withAlphaComponent(0.5)
////    store[NTColor.Name("NTtoken-btn-selected-text-danger")] = NTColor.R600 & NTColor.R500
////    store[NTColor.Name("NTtoken-btn-selected-text-success")] = NTColor.G500
////    store[NTColor.Name("NTtoken-component-outlined-bg")] = NTColor.bgBody & NTColor.N00.withAlphaComponent(0.0)
////    store[NTColor.Name("NTtoken-component-text-disabled-loading")] = NTColor.N500
////    store[NTColor.Name("NTtoken-dropdown-button-divider")] = NTColor.N00.withAlphaComponent(0.5) & NTColor.lineBorderComponent
////    store[NTColor.Name("NTtoken-input-bg-disabled")] = NTColor.N200 & NTColor.N900.withAlphaComponent(0.12)
////    store[NTColor.Name("NTtoken-message-card-bg-blue")] = NTColor.B100
////    store[NTColor.Name("NTtoken-message-card-bg-carmine")] = NTColor.C100
////    store[NTColor.Name("NTtoken-message-card-bg-green")] = NTColor.G100
////    store[NTColor.Name("NTtoken-message-card-bg-indigo")] = NTColor.I100
////    store[NTColor.Name("NTtoken-message-card-bg-light-neutral")] = NTColor.N100 & NTColor.N200
////    store[NTColor.Name("NTtoken-message-card-bg-lime")] = NTColor.L100
////    store[NTColor.Name("NTtoken-message-card-bg-mask-general")] = NTColor.N00.withAlphaComponent(0.5) & NTColor.N00.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-message-card-bg-mask-special")] = NTColor.N00.withAlphaComponent(0.2) & NTColor.N00.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-message-card-bg-neutral")] = NTColor.N500 & NTColor.N300
////    store[NTColor.Name("NTtoken-message-card-bg-orange")] = NTColor.O100
////    store[NTColor.Name("NTtoken-message-card-bg-purple")] = NTColor.P100
////    store[NTColor.Name("NTtoken-message-card-bg-red")] = NTColor.R100
////    store[NTColor.Name("NTtoken-message-card-bg-turquoise")] = NTColor.T100
////    store[NTColor.Name("NTtoken-message-card-bg-violet")] = NTColor.V100
////    store[NTColor.Name("NTtoken-message-card-bg-wathet")] = NTColor.W100
////    store[NTColor.Name("NTtoken-message-card-bg-yellow")] = NTColor.Y100
////    store[NTColor.Name("NTtoken-message-card-text-blue")] = NTColor.B600 & NTColor.B700
////    store[NTColor.Name("NTtoken-message-card-text-carmine")] = NTColor.C600 & NTColor.C700
////    store[NTColor.Name("NTtoken-message-card-text-green")] = NTColor.G600 & NTColor.G700
////    store[NTColor.Name("NTtoken-message-card-text-indigo")] = NTColor.I600 & NTColor.I700
////    store[NTColor.Name("NTtoken-message-card-text-lime")] = NTColor.L600 & NTColor.L700
////    store[NTColor.Name("NTtoken-message-card-text-neutral")] = NTColor.N00 & NTColor.N650
////    store[NTColor.Name("NTtoken-message-card-text-orange")] = NTColor.O500 & NTColor.O700
////    store[NTColor.Name("NTtoken-message-card-text-purple")] = NTColor.P600 & NTColor.P700
////    store[NTColor.Name("NTtoken-message-card-text-red")] = NTColor.R600 & NTColor.R700
////    store[NTColor.Name("NTtoken-message-card-text-turquoise")] = NTColor.T600 & NTColor.T700
////    store[NTColor.Name("NTtoken-message-card-text-violet")] = NTColor.V600 & NTColor.V700
////    store[NTColor.Name("NTtoken-message-card-text-wathet")] = NTColor.W600 & NTColor.W700
////    store[NTColor.Name("NTtoken-message-card-text-yellow")] = NTColor.Y500 & NTColor.Y700
////    store[NTColor.Name("NTtoken-navigation-bar-resize-handle")] = NTColor.rgb(0x4599ff)
////    store[NTColor.Name("NTtoken-navigation-bar-tab-bg-black")] = NTColor.rgb(0x1c2435) & NTColor.rgb(0x0d0e12)
////    store[NTColor.Name("NTtoken-navigation-bar-tab-bg-blue")] = NTColor.rgb(0x1b429c) & NTColor.rgb(0x141c2a)
////    store[NTColor.Name("NTtoken-navigation-bar-tab-bg-gray")] = NTColor.rgb(0xe5e5e9) & NTColor.rgb(0x242424)
////    store[NTColor.Name("NTtoken-progress-bg")] = NTColor.N300 & NTColor.N900.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-quote-bar-bg")] = NTColor.N400
////    store[NTColor.Name("NTtoken-reaction-bg-grey")] = NTColor.N900.withAlphaComponent(0.05) & NTColor.rgb(0x101010)
////    store[NTColor.Name("NTtoken-reaction-bg-grey-float")] = NTColor.N900.withAlphaComponent(0.05) & NTColor.rgb(0x1f1f1f)
////    store[NTColor.Name("NTtoken-skeleton-bg")] = NTColor.N900.withAlphaComponent(0.08) & NTColor.rgb(0xf0f0f0).withAlphaComponent(0.08)
////    store[NTColor.Name("NTtoken-skeleton-fg")] = NTColor.N900.withAlphaComponent(0.05) & NTColor.rgb(0xf0f0f0).withAlphaComponent(0.05)
////    store[NTColor.Name("NTtoken-sliding-block-bg-disabled-loading")] = NTColor.N00 & NTColor.N500
////    store[NTColor.Name("NTtoken-steps-bg-hover")] = NTColor.primaryFillTransparent01
////    store[NTColor.Name("NTtoken-tab-pri-bg")] = NTColor.primaryFillTransparent03 & NTColor.primaryFillSolid03
////    store[NTColor.Name("NTtoken-tab-pri-text")] = NTColor.primaryContentDefault
////    store[NTColor.Name("NTtoken-tab-se-bg-unselected")] = NTColor.N900.withAlphaComponent(0.1) & NTColor.N900.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-table-bg-expand")] = NTColor.N100 & NTColor.N00
////    store[NTColor.Name("NTtoken-table-bg-grey")] = NTColor.N50 & NTColor.N100
////    store[NTColor.Name("NTtoken-table-bg-head")] = NTColor.N100 & NTColor.N200
////    store[NTColor.Name("NTtoken-table-bg-hover")] = NTColor.N200 & NTColor.N350
////    store[NTColor.Name("NTtoken-table-bg-pressed")] = NTColor.N300 & NTColor.N400
////    store[NTColor.Name("NTtoken-table-bg-selected")] = NTColor.primaryFillSolid01 & NTColor.primaryFillSolid02
////    store[NTColor.Name("NTtoken-tag-bg-blue")] = NTColor.B600.withAlphaComponent(0.2) & NTColor.B500.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-blue-solid")] = NTColor.B200 & NTColor.B100
////    store[NTColor.Name("NTtoken-tag-bg-blue-default")] = NTColor.B600.withAlphaComponent(0.12)
////    store[NTColor.Name("NTtoken-tag-bg-blue-hover")] = NTColor.B600.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-blue-pressed")] = NTColor.B600.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-carmine")] = NTColor.C500.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-carmine-solid")] = NTColor.C200 & NTColor.C100
////    store[NTColor.Name("NTtoken-tag-bg-carmine-hover")] = NTColor.C500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-green")] = NTColor.G350.withAlphaComponent(0.2) & NTColor.G500.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-green-solid")] = NTColor.G200 & NTColor.G100
////    store[NTColor.Name("NTtoken-tag-bg-green-hover")] = NTColor.G350.withAlphaComponent(0.3) & NTColor.G500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-indigo")] = NTColor.I500.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-indigo-solid")] = NTColor.I200 & NTColor.I100
////    store[NTColor.Name("NTtoken-tag-bg-indigo-hover")] = NTColor.I500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-lime")] = NTColor.L350.withAlphaComponent(0.2) & NTColor.L500.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-lime-solid")] = NTColor.L200 & NTColor.L100
////    store[NTColor.Name("NTtoken-tag-bg-lime-hover")] = NTColor.L350.withAlphaComponent(0.3) & NTColor.L500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-orange")] = NTColor.O350.withAlphaComponent(0.15) & NTColor.O500.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-orange-solid")] = NTColor.O200 & NTColor.O100
////    store[NTColor.Name("NTtoken-tag-bg-orange-hover")] = NTColor.O350.withAlphaComponent(0.3) & NTColor.O500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-orange-pressed")] = NTColor.O350.withAlphaComponent(0.35) & NTColor.O500.withAlphaComponent(0.35)
////    store[NTColor.Name("NTtoken-tag-bg-purple")] = NTColor.P500.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-purple-solid")] = NTColor.P200 & NTColor.P100
////    store[NTColor.Name("NTtoken-tag-bg-purple-hover")] = NTColor.P500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-red")] = NTColor.R400.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-red-solid")] = NTColor.R200 & NTColor.R100
////    store[NTColor.Name("NTtoken-tag-bg-red-hover")] = NTColor.R400.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-turquoise")] = NTColor.T300.withAlphaComponent(0.2) & NTColor.T500.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-turquoise-solid")] = NTColor.T200 & NTColor.T100
////    store[NTColor.Name("NTtoken-tag-bg-turquoise-hover")] = NTColor.T300.withAlphaComponent(0.3) & NTColor.T500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-violet")] = NTColor.V500.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-violet-solid")] = NTColor.V200 & NTColor.V100
////    store[NTColor.Name("NTtoken-tag-bg-violet-hover")] = NTColor.V500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-wathet")] = NTColor.W300.withAlphaComponent(0.2) & NTColor.W500.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-bg-wathet-solid")] = NTColor.W200 & NTColor.W100
////    store[NTColor.Name("NTtoken-tag-bg-wathet-hover")] = NTColor.W300.withAlphaComponent(0.3) & NTColor.W500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-bg-yellow")] = NTColor.Y300.withAlphaComponent(0.2) & NTColor.Y400.withAlphaComponent(0.15)
////    store[NTColor.Name("NTtoken-tag-bg-yellow-solid")] = NTColor.Y200 & NTColor.Y100
////    store[NTColor.Name("NTtoken-tag-bg-yellow-hover")] = NTColor.Y300.withAlphaComponent(0.4) & NTColor.Y500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-close-blue-disabled")] = NTColor.B600.withAlphaComponent(0.8)
////    store[NTColor.Name("NTtoken-tag-close-blue-hover")] = NTColor.B600
////    store[NTColor.Name("NTtoken-tag-close-blue-normal")] = NTColor.B600.withAlphaComponent(0.6)
////    store[NTColor.Name("NTtoken-tag-close-blue-pressed")] = NTColor.B600.withAlphaComponent(0.8)
////    store[NTColor.Name("NTtoken-tag-close-orange-disabled")] = NTColor.O350.withAlphaComponent(0.3) & NTColor.O500.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-close-orange-hover")] = NTColor.O350 & NTColor.O500
////    store[NTColor.Name("NTtoken-tag-close-orange-normal")] = NTColor.O350.withAlphaComponent(0.6) & NTColor.O500.withAlphaComponent(0.6)
////    store[NTColor.Name("NTtoken-tag-close-orange-pressed")] = NTColor.O350.withAlphaComponent(0.8) & NTColor.O500.withAlphaComponent(0.8)
////    store[NTColor.Name("NTtoken-tag-neutral-bg-inverse")] = NTColor.N500 & NTColor.N400
////    store[NTColor.Name("NTtoken-tag-neutral-bg-inverse-opacity")] = NTColor.N900.withAlphaComponent(0.4)
////    store[NTColor.Name("NTtoken-tag-neutral-bg-normal")] = NTColor.N900.withAlphaComponent(0.1) & NTColor.N600.withAlphaComponent(0.2)
////    store[NTColor.Name("NTtoken-tag-neutral-bg-normal-hover")] = NTColor.N900.withAlphaComponent(0.15) & NTColor.N900.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-neutral-bg-normal-pressed")] = NTColor.N900.withAlphaComponent(0.2) & NTColor.N900.withAlphaComponent(0.4)
////    store[NTColor.Name("NTtoken-tag-neutral-bg-solid")] = NTColor.N200
////    store[NTColor.Name("NTtoken-tag-neutral-close-disabled")] = NTColor.textTitle.withAlphaComponent(0.3)
////    store[NTColor.Name("NTtoken-tag-neutral-close-hover")] = NTColor.textTitle
////    store[NTColor.Name("NTtoken-tag-neutral-close-normal")] = NTColor.textTitle.withAlphaComponent(0.6)
////    store[NTColor.Name("NTtoken-tag-neutral-close-pressed")] = NTColor.textTitle.withAlphaComponent(0.8)
////    store[NTColor.Name("NTtoken-tag-neutral-text-inverse")] = NTColor.N00 & NTColor.N1000
////    store[NTColor.Name("NTtoken-tag-neutral-text-inverse-opacity")] = NTColor.N00 & NTColor.N1000
////    store[NTColor.Name("NTtoken-tag-neutral-text-normal")] = NTColor.N600
////    store[NTColor.Name("NTtoken-tag-neutral-text-solid")] = NTColor.N600
////    store[NTColor.Name("NTtoken-tag-state-blue-light")] = NTColor.B500
////    store[NTColor.Name("NTtoken-tag-state-green-light")] = NTColor.G500
////    store[NTColor.Name("NTtoken-tag-state-grey-light")] = NTColor.N500
////    store[NTColor.Name("NTtoken-tag-state-orange-light")] = NTColor.O350 & NTColor.O500
////    store[NTColor.Name("NTtoken-tag-state-purple-light")] = NTColor.P500
////    store[NTColor.Name("NTtoken-tag-state-red-light")] = NTColor.R500
////    store[NTColor.Name("NTtoken-tag-text-s-blue")] = NTColor.B600 & NTColor.B700
////    store[NTColor.Name("NTtoken-tag-text-s-carmine")] = NTColor.C600 & NTColor.C700
////    store[NTColor.Name("NTtoken-tag-text-s-green")] = NTColor.G600 & NTColor.G700
////    store[NTColor.Name("NTtoken-tag-text-s-indigo")] = NTColor.I600 & NTColor.I700
////    store[NTColor.Name("NTtoken-tag-text-s-lime")] = NTColor.L600 & NTColor.L700
////    store[NTColor.Name("NTtoken-tag-text-s-orange")] = NTColor.O500 & NTColor.O700
////    store[NTColor.Name("NTtoken-tag-text-s-purple")] = NTColor.P600 & NTColor.P700
////    store[NTColor.Name("NTtoken-tag-text-s-red")] = NTColor.R600 & NTColor.R700
////    store[NTColor.Name("NTtoken-tag-text-s-turquoise")] = NTColor.T600 & NTColor.T700
////    store[NTColor.Name("NTtoken-tag-text-s-violet")] = NTColor.V600 & NTColor.V700
////    store[NTColor.Name("NTtoken-tag-text-s-wathet")] = NTColor.W600 & NTColor.W700
////    store[NTColor.Name("NTtoken-tag-text-s-yellow")] = NTColor.Y500 & NTColor.Y700
////    store[NTColor.Name("NTtoken-tag-text-blue")] = NTColor.B900 & NTColor.B700
////    store[NTColor.Name("NTtoken-tag-text-carmine")] = NTColor.C900 & NTColor.C700
////    store[NTColor.Name("NTtoken-tag-text-green")] = NTColor.G900 & NTColor.G700
////    store[NTColor.Name("NTtoken-tag-text-indigo")] = NTColor.I900 & NTColor.I700
////    store[NTColor.Name("NTtoken-tag-text-lime")] = NTColor.L900 & NTColor.L700
////    store[NTColor.Name("NTtoken-tag-text-orange")] = NTColor.O900 & NTColor.O700
////    store[NTColor.Name("NTtoken-tag-text-purple")] = NTColor.P900 & NTColor.P700
////    store[NTColor.Name("NTtoken-tag-text-red")] = NTColor.R900 & NTColor.R700
////    store[NTColor.Name("NTtoken-tag-text-turquoise")] = NTColor.T900 & NTColor.T700
////    store[NTColor.Name("NTtoken-tag-text-violet")] = NTColor.V900 & NTColor.V700
////    store[NTColor.Name("NTtoken-tag-text-wathet")] = NTColor.W900 & NTColor.W700
////    store[NTColor.Name("NTtoken-tag-text-yellow")] = NTColor.Y900 & NTColor.Y700
////    store[NTColor.Name("NTtoken-upload-bg-error")] = NTColor.N200 & NTColor.N900.withAlphaComponent(0.12)
////    store[NTColor.Name("NTtoken-upload-mask-img")] = NTColor.N900.withAlphaComponent(0.4) & NTColor.rgb(0x000000).withAlphaComponent(0.7)
////    store[NTColor.Name("NTtoken-colorpicker-carmine")] = NTColor.C400
////    store[NTColor.Name("NTtoken-colorpicker-red")] = NTColor.R400 & NTColor.R500
////    store[NTColor.Name("NTtoken-colorpicker-orange")] = NTColor.O350 & NTColor.O700
////    store[NTColor.Name("NTtoken-colorpicker-yellow")] = NTColor.Y350 & NTColor.Y600
////    store[NTColor.Name("NTtoken-colorpicker-green")] = NTColor.G350 & NTColor.G600
////    store[NTColor.Name("NTtoken-colorpicker-turquoise")] = NTColor.T300 & NTColor.T700
////    store[NTColor.Name("NTtoken-colorpicker-blue")] = NTColor.B500
////    store[NTColor.Name("NTtoken-colorpicker-wathet")] = NTColor.W300 & NTColor.W700
////    store[NTColor.Name("NTtoken-colorpicker-indigo")] = NTColor.I600 & NTColor.I400
////    store[NTColor.Name("NTtoken-colorpicker-purple")] = NTColor.P600 & NTColor.P400
////    store[NTColor.Name("NTtoken-colorpicker-violet")] = NTColor.V500
////    store[NTColor.Name("NTtoken-colorpicker-sunflower")] = NTColor.S200 & NTColor.S700
////    store[NTColor.Name("NTtoken-colorpicker-lime")] = NTColor.L300 & NTColor.L700
////    store[NTColor.Name("NTtoken-colorpicker-neutral")] = NTColor.N500
////    store[NTColor.Name("NTtoken-switch-handle-disabled")] = NTColor.staticWhite.withAlphaComponent(0.6) & NTColor.staticWhite.withAlphaComponent(0.3)
////    return store
//// }
////// swiftlint:enable all
//// }
