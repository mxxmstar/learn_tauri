//! HTTP 客户端错误类型
//!
//! 统一封装 HTTP 请求中可能出现的各种错误，方便调用方集中处理。

use thiserror::Error;

/// HTTP 客户端错误枚举
///
/// 覆盖了 HTTP 请求全生命周期中可能出现的错误场景：
/// - 构建请求时的参数错误
/// - 网络传输过程中的连接/超时错误
/// - 服务端返回的非成功状态码
/// - 响应体解析错误
#[derive(Error, Debug)]
pub enum HttpClientError {
    /// 请求构建失败
    ///
    /// 通常发生在 URL 格式错误、请求头格式不合法等场景。
    #[error("请求构建失败: {0}")]
    RequestBuildError(String),

    /// 网络请求执行失败
    ///
    /// 包括 DNS 解析失败、连接被拒绝、TLS 握手失败等底层网络问题。
    #[error("网络请求失败: {0}")]
    RequestError(String),

    /// 服务端返回错误状态码
    ///
    /// 服务端正常响应但 HTTP 状态码表示错误（4xx/5xx）。
    /// 包含状态码和响应体内容，方便调用方排查问题。
    #[error("服务端返回错误状态码 {status}: {body}")]
    HttpStatusError {
        /// HTTP 状态码
        status: u16,
        /// 响应体文本
        body: String,
    },

    /// 响应体序列化/反序列化失败
    ///
    /// 当响应体不是合法的 JSON 格式时触发此错误。
    #[error("响应解析失败: {0}")]
    ResponseParseError(String),

    /// URL 解析失败
    ///
    /// 传入的 URL 字符串不符合 RFC 3986 规范时触发。
    #[error("URL 解析失败: {0}")]
    UrlParseError(String),

    /// 超时错误
    ///
    /// 请求超过指定的超时时间仍未完成时触发。
    #[error("请求超时: {0}")]
    TimeoutError(String),

    /// TLS/SSL 错误
    ///
    /// HTTPS 连接中证书验证失败等 TLS 层面的错误。
    #[error("TLS/SSL 错误: {0}")]
    TlsError(String),
}

/// 方便将 `reqwest::Error` 转换为 `HttpClientError`
impl From<reqwest::Error> for HttpClientError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            HttpClientError::TimeoutError(e.to_string())
        } else if e.is_builder() {
            HttpClientError::RequestBuildError(e.to_string())
        } else if e.is_connect() {
            HttpClientError::RequestError(format!("连接失败: {}", e))
        } else if e.is_decode() {
            HttpClientError::ResponseParseError(e.to_string())
        } else {
            HttpClientError::RequestError(e.to_string())
        }
    }
}
