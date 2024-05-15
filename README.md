# JsonOrProtobuf

Simple Axum Extractor that extracts both Json & Protobuf allowing both `Content-Type`'s. Can also use the `decompose()` method to track the `Content-Type` specified, and `from_accept_header` to follow `Accept` header.