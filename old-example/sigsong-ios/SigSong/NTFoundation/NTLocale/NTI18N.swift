//
// NTI18N.swift
// swiftuidemo
//
// Created by Zigeng on 2024/1/25.
//

import Foundation

extension Locale: RawRepresentable {
    public var rawValue: String { return identifier }
    // here is the known language, helper case constant
    // swiftlint:disable identifier_name
    public static let id_ID = Locale(rawValue: "id_ID") // 印尼文（Bahasa）
    public static let de_DE = Locale(rawValue: "de_DE") // 德文
    public static let en_US = Locale(rawValue: "en_US") // 英文
    public static let es_ES = Locale(rawValue: "es_ES") // 西班牙文
    public static let fr_FR = Locale(rawValue: "fr_FR") // 法文
    public static let it_IT = Locale(rawValue: "it_IT") // 意大利文
    public static let pt_BR = Locale(rawValue: "pt_BR") // 葡萄牙文（巴西）
    public static let vi_VN = Locale(rawValue: "vi_VN") // 越南文
    public static let ru_RU = Locale(rawValue: "ru_RU") // 俄文
    public static let hi_IN = Locale(rawValue: "hi_IN") // 印地文
    public static let th_TH = Locale(rawValue: "th_TH") // 泰文
    public static let ko_KR = Locale(rawValue: "ko_KR") // 韩文
    public static let zh_CN = Locale(rawValue: "zh_CN") // 中文
    public static let zh_TW = Locale(rawValue: "zh_TW") // 台湾繁体中文
    public static let zh_HK = Locale(rawValue: "zh_HK") // 香港繁体中文
    public static let ja_JP = Locale(rawValue: "ja_JP") // 日文
    public static let ms_MY = Locale(rawValue: "ms_MY") // 马来西亚语

    // swiftlint:enable identifier_name
    public init(rawValue: String) {
        // ensure region concat with _
        self = Locale(identifier: rawValue.replacingOccurrences(of: "-", with: "_"))
    }
    public var displayName: String {
        switch self {
        case .id_ID: return "Bahasa Indonesia"
        case .de_DE: return "Deutsch"
        case .en_US: return "English"
        case .es_ES: return "Español"
        case .fr_FR: return "Français"
        case .it_IT: return "Italiano"
        case .pt_BR: return "Português (Brasil)"
        case .vi_VN: return "Tiếng Việt "
        case .ru_RU: return "Русский"
        case .hi_IN: return "हिन्दी"
        case .th_TH: return "ภาษาไทย "
        case .ko_KR: return "한국어"
        case .zh_CN: return "简体中文"
        #if OVERSEA
        case .zh_HK: return "繁體中文 (香港)"
        case .zh_TW: return "繁體中文 (台灣)"
        #else
        case .zh_HK: return "繁體中文 (中国香港)"
        case .zh_TW: return "繁體中文 (中国台灣)"
        #endif

        case .ja_JP: return "日本語"
        default:
            // 默认使用语言自身来描述自己的语言
            return localizedString(forLanguageCode: identifier) ?? ""
        }
    }
    var identifier: String {
        return identifier(.bcp47)
    }
}
