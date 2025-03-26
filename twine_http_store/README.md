# twine_http_store

[![Crates.io Version](https://img.shields.io/crates/v/twine_http_store)](https://crates.io/crates/twine_http_store)
[![docs.rs (with version)](https://img.shields.io/docsrs/twine_http_store/latest)](https://docs.rs/twine_http_store/latest/twine_http_store/)

This crate provides a standard way to save twine data to a remote data source
through the twine HTTP api.

## A note on authorization

When using an HTTP api, different services may have different restrictions
for saving data (like the need for API keys). The underlying [`reqwest::Client`]
can be customized to accommodate these situations.

## Versions

There are currently two versions of the HTTP api. If you know which one you
are dealing with, use the [`v1`] or [`v2`] modules accordingly. If you can't
predict which you will work with you can use [`determine_version`].

## Streaming

Calls that involve streams will be batched into descrete requests for
robustness.

## Examples

See the [examples](https://github.com/twine-protocol/twine-rs/tree/main/twine_http_store/examples) for example uses.
