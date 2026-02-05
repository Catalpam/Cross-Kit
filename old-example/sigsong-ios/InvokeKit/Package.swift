// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "InvokeKit",
    platforms: [
        .iOS(.v16) // 设置最低支持的iOS版本为iOS 13.0
    ],
    products: [
        // Products define the executables and libraries a package produces, making them visible to other packages.
        .library(
            name: "InvokeKit",
            targets: ["InvokeKit"]),
    ], 
    dependencies: [
        .package(name: "SigsongSDK", path: "../SigsongSDK")
    ],
    targets: [
        // Targets are the basic building blocks of a package, defining a module or a test suite.
        // Targets can depend on other targets in this package and products from dependencies.
        .target(
            name: "InvokeKit",
            dependencies: [
                "SigsongSDK"
            ]),
        .testTarget(
            name: "InvokeKitTests",
            dependencies: ["InvokeKit"]),
    ]
)

//POST /jsonapi_s HTTP/1.1
//Content-Type: application/x-www-form-urlencoded
//Accept: application/json, text/plain, */*
//Sec-Fetch-Site: same-site
//Accept-Language: zh-CN,zh-Hans;q=0.9
//Accept-Encoding: gzip, deflate, br
//Sec-Fetch-Mode: cors
//Host: dict.youdao.com
//Origin: https://www.youdao.com
//User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15
//Referer: https://www.youdao.com/
//Content-Length: 104
//Connection: keep-alive
//Sec-Fetch-Dest: empty
//Cookie: OUTFOX_SEARCH_USER_ID=711630588@115.171.85.79; OUTFOX_SEARCH_USER_ID_NCOO=1457749295.3510406
//
//                                        curl 'https://dict.youdao.com/jsonapi_s?doctype=json&jsonversion=4' \
//                                        -X 'POST' \
//                                        -H 'Content-Type: application/x-www-form-urlencoded' \
//                                        -H 'Accept: application/json, text/plain, */*' \
//                                        -H 'Sec-Fetch-Site: same-site' \
//                                        -H 'Accept-Language: zh-CN,zh-Hans;q=0.9' \
//                                        -H 'Accept-Encoding: gzip, deflate, br' \
//                                        -H 'Sec-Fetch-Mode: cors' \
//                                        -H 'Host: dict.youdao.com' \
//                                        -H 'Origin: https://www.youdao.com' \
//                                        -H 'User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15' \
//                                        -H 'Referer: https://www.youdao.com/' \
//                                        -H 'Content-Length: 104' \
//                                        -H 'Connection: keep-alive' \
//                                        -H 'Sec-Fetch-Dest: empty' \
//                                        -H 'Cookie: OUTFOX_SEARCH_USER_ID=711630588@115.171.85.79; OUTFOX_SEARCH_USER_ID_NCOO=1457749295.3510406' \
//                                        --data 'q=%E6%8F%8F%E3%81%84%E3%81%9F&le=ja&t=0&client=web&sign=ec34e5f3057c66c216633ad71f0ce2b8&keyfrom=webdict'
