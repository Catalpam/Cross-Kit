#[macro_export]
macro_rules! impl_mapping {
    ($request_type:ty, $response_type:ty, $request_variant:path, $response_variant:path) => {
        impl ToCommonRequest for $request_type {
            fn into_common_request(self) -> Request {
                $request_variant(self)
            }
        }
        impl FromCommonResponse for $response_type {
            fn from_common_response(response: Response) -> Option<Self> {
                if let $response_variant(response) = response { Some(response) } else { None }
            }
        }
    };
}
