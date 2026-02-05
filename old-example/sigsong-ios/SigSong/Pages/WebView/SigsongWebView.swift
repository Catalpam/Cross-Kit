////
////  SigsongWebView.swift
////  SigSong
////
////  Created by zigengm3 on 2024/9/1.
////
//
//import Foundation
//import SwiftUI
//import WebKit
//import SigsongSDK
//import InvokeKit
//
//struct SigSongWebView: UIViewRepresentable {
//
//    func makeCoordinator() -> SigSongWebViewCoordinator {
//        return Coordinator()
//    }
//
//    func makeUIView(context: Context) -> WKWebView {
//        let coordinator = makeCoordinator()
//        let userContentController = WKUserContentController()
//
//        userContentController.add(coordinator, name: "unify")
//        let configuration = WKWebViewConfiguration()
//
//        let preferences = WKPreferences()
//
//        preferences.javaScriptCanOpenWindowsAutomatically = true
//        configuration.preferences = preferences
//
//        configuration.userContentController = userContentController
//        let webview = WKWebView(frame: .zero, configuration: configuration)
//        webview.navigationDelegate = coordinator
//        webview.scrollView.delegate = coordinator
//        webview.scrollView.showsVerticalScrollIndicator = false
//        webview.scrollView.showsHorizontalScrollIndicator = false
////        // 设置 webView 的自动调整约束，忽略安全区域
//        webview.translatesAutoresizingMaskIntoConstraints = false
////        NSLayoutConstraint.activate([
////            webview.topAnchor.constraint(equalTo: view.topAnchor),
////            webview.bottomAnchor.constraint(equalTo: view.bottomAnchor), // 忽略底部安全区
////            webview.leadingAnchor.constraint(equalTo: view.leadingAnchor),
////            webview.trailingAnchor.constraint(equalTo: view.trailingAnchor)
////        ])
////        #if DEBUG
//        webview.isInspectable = true
//        webview.load(URLRequest(url: URL(string: "http://192.168.31.193:5173/")!))
////        #else
////        webview.loadHTMLString(coordinator, baseURL: nil)
////        #endif
//
//        return webview
//    }
//
//    func updateUIView(_ webView: WKWebView, context: Context) {
//    }
//}
//
//class SigSongWebViewCoordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler, UIScrollViewDelegate {
//    override init() {
//        let client = ClientWebViewImpl()
//        self.webSDK = API.getWebviewSdk(webView: client)
//        self.client = client
//    }
//
//    let webSDK: WebViewSdk
//    let client: ClientWebViewImpl
//
//    var webView: WKWebView? {
//        didSet {
//            client.webView = self.webView
//        }
//    }
//
//    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
//        self.webView = webView
//        webView.scrollView.delegate = self
//    }
//
//    // WKUIDelegate 方法，禁用双击放大网页功能
//    func webView(_ webView: WKWebView, shouldPreviewElement elementInfo: WKContextMenuElementInfo) -> Bool {
//        return false
//    }
//
//    // UIScrollViewDelegate 方法，禁用双击放大网页功能
//    func viewForZooming(in scrollView: UIScrollView) -> UIView? {
//        return nil
//    }
//
//    // receive message from wkwebview
//    func userContentController(
//        _ userContentController: WKUserContentController,
//        didReceive message: WKScriptMessage
//    ) {
//        let date = Date()
//        if let body = message.body as? String {
//            try? webSDK.request(json: body)
//        }
//    }
//
//    func webView(_ webView: WKWebView,
//                 decidePolicyFor navigationAction: WKNavigationAction,
//                 decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
//        decisionHandler(.allow)
//    }
//
//    func messageToWebview(msg: String) {
//        self.webView?.evaluateJavaScript("console.log('begin')")
//        self.webView?.evaluateJavaScript("window.onMessage('\(msg)')")
//        self.webView?.evaluateJavaScript("console.log('end')")
//    }
//}
//
//class ClientWebViewImpl: ClientWebView {
//    weak var webView: WKWebView?
//    func evalJavascript(script: String) {
//        self.webView?.evaluateJavaScript(script)
//    }
//
//    func pushResponse(command: String, requestId: String, json: String) {
//        print(command)
//        print(json)
//        self.webView?.evaluateJavaScript("window.onMessage({command:\"\(command)\",request_id:\"\(requestId)\",data: \(json)})")
//    }
//}
