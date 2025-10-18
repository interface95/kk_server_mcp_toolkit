use aes::Aes128;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use cbc::{Decryptor, cipher::{BlockDecryptMut, KeyIvInit}};
use flate2::read::GzDecoder;
use hex::{FromHex, encode as hex_encode};
use prost::Message;
use rmcp::{
    ErrorData, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities,
        ServerInfo,
    },
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{fs, io::Read, path::PathBuf};

mod generated;

use generated::com::kuaishou::client::log::BatchReportEvent;

#[derive(Debug, Serialize)]
pub struct CallResult<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> CallResult<T> {
    fn ok(message: impl Into<String>, data: T) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    fn fail(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BatchReportEventDto {
    pub data: Value,
    pub decompressed_hex: String,
    pub input_md5: String,
    pub result_md5: String,
    pub is_match: bool,
}

#[derive(Debug, Serialize)]
pub struct BatchReportEventParseResult {
    pub parsed: Option<BatchReportEventDto>,
    pub steps: Vec<String>,
}

fn strip_nulls(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|_, v| {
                strip_nulls(v);
                !v.is_null()
            });
        }
        Value::Array(arr) => {
            arr.iter_mut().for_each(strip_nulls);
            arr.retain(|v| !v.is_null());
        }
        _ => {}
    }
}

// 请求参数结构体
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParseBase64Request {
    pub base64_data: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParseHexRequest {
    pub hex_data: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParseFileRequest {
    pub path: String,
}

#[derive(Clone)]
pub struct ToolkitServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl ToolkitServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// 通过 Base64 编码的 gzip 压缩数据解析 BatchReportEvent
    #[tool(description = "解析 Base64 编码的 BatchReportEvent (gzip)")]
    fn parse_batch_from_base64(
        &self,
        Parameters(ParseBase64Request { base64_data }): Parameters<ParseBase64Request>,
    ) -> Result<CallToolResult, ErrorData> {
        match decode_base64(&base64_data) {
            Ok(bytes) => respond_with_parse_result(parse_batch_from_bytes(bytes)),
            Err(err) => {
                let json_val = serde_json::to_value(CallResult::<BatchReportEventDto>::fail(err))
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::json(json_val)?]))
            }
        }
    }

    /// 通过十六进制字符串解析 BatchReportEvent（gzip）
    #[tool(description = "解析十六进制字符串的 BatchReportEvent (gzip)")]
    fn parse_batch_from_hex(
        &self,
        Parameters(ParseHexRequest { hex_data }): Parameters<ParseHexRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        match decode_hex(&hex_data) {
            Ok(bytes) => respond_with_parse_result(parse_batch_from_bytes(bytes)),
            Err(err) => {
                let json_val = serde_json::to_value(CallResult::<BatchReportEventDto>::fail(err))
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::json(json_val)?]))
            }
        }
    }

    /// 通过文件路径解析 BatchReportEvent（gzip）
    #[tool(description = "从文件路径解析 BatchReportEvent (gzip)")]
    fn parse_batch_from_file(
        &self,
        Parameters(ParseFileRequest { path }): Parameters<ParseFileRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        if path.trim().is_empty() {
            let json_val = serde_json::to_value(CallResult::<BatchReportEventDto>::fail(
                "提供的文件路径为空。",
            ))
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
            return Ok(CallToolResult::success(vec![Content::json(json_val)?]));
        }

        let path_buf = PathBuf::from(&path);
        if !path_buf.exists() {
            let json_val = serde_json::to_value(CallResult::<BatchReportEventDto>::fail(format!(
                "文件不存在：{}",
                path
            )))
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
            return Ok(CallToolResult::success(vec![Content::json(json_val)?]));
        }

        match fs::read(&path_buf) {
            Ok(bytes) => respond_with_parse_result(parse_batch_from_bytes(bytes)),
            Err(err) => {
                let json_val = serde_json::to_value(CallResult::<BatchReportEventDto>::fail(format!(
                    "读取文件失败：{}",
                    err
                )))
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::json(json_val)?]))
            }
        }
    }
}

fn respond_with_parse_result(
    result: BatchReportEventParseResult,
) -> Result<CallToolResult, ErrorData> {
    let response = match result.parsed {
        Some(parsed) => CallResult::ok(
            format!("解析成功，尝试步骤：{}", result.steps.join(" -> ")),
            parsed,
        ),
        None => CallResult::fail(format!("解析失败，尝试步骤：{}", result.steps.join(" -> "))),
    };

    let json_val = serde_json::to_value(response)
        .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::json(json_val)?]))
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    BASE64_STANDARD
        .decode(input)
        .map_err(|err| format!("输入不是有效的 Base64 编码：{err}"))
}

fn decode_hex(input: &str) -> Result<Vec<u8>, String> {
    Vec::from_hex(input).map_err(|err| format!("输入不是有效的十六进制编码：{err}"))
}

fn parse_batch_from_bytes(bytes: Vec<u8>) -> BatchReportEventParseResult {
    // 策略 1: 普通 gzip 解压
    if let Some(result) =
        try_parse_with_strategy(&bytes, "CommonUtility::GzipDecompress", |input| {
            let mut decoder = GzDecoder::new(&input[..]);
            let mut data = Vec::new();
            decoder.read_to_end(&mut data).map(|_| data)
        })
    {
        return result;
    }

    // 策略 2: Gzip2 解压（AES 解密 + Gzip 解压）
    if let Some(result) =
        try_parse_with_strategy(&bytes, "AppSecurity::Gzip2Decompress", gzip2_decompress)
    {
        return result;
    }

    // 策略 3: 直接解析（无需解压）
    if let Some(result) = try_parse_with_strategy(&bytes, "直接解析", |input| Ok(input.to_vec()))
    {
        return result;
    }

    BatchReportEventParseResult {
        parsed: None,
        steps: vec![
            "步骤失败：CommonUtility::GzipDecompress".to_string(),
            "步骤失败：AppSecurity::Gzip2Decompress".to_string(),
            "步骤失败：直接解析".to_string(),
        ],
    }
}

/// Gzip2 解压：先 AES 解密，再 Gzip 解压
///
/// 对应 C# 代码：
fn gzip2_decompress(input: &[u8]) -> std::io::Result<Vec<u8>> {
    // AES 密钥和 IV（来自 AppSecurity.cs 常量定义）
    // ConstDefaultKey = "46a8qpMw6643TDiV"
    // ConstDefaultIv = "W3HaJGyGrfOVRb42"
    const AES_KEY: &[u8; 16] = b"46a8qpMw6643TDiV";
    const AES_IV: &[u8; 16] = b"W3HaJGyGrfOVRb42";

    // 1. AES-128-CBC 解密（PKCS7 填充）
    type Aes128CbcDec = Decryptor<Aes128>;

    let mut buffer = input.to_vec();

    let decrypted = Aes128CbcDec::new(AES_KEY.into(), AES_IV.into())
        .decrypt_padded_mut::<cbc::cipher::block_padding::Pkcs7>(&mut buffer)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("AES 解密失败: {:?}", e)))?;

    // 2. Gzip 解压
    let mut decoder = GzDecoder::new(&decrypted[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    Ok(decompressed)
}

fn try_parse_with_strategy<F>(
    input: &[u8],
    name: &str,
    transform: F,
) -> Option<BatchReportEventParseResult>
where
    F: Fn(&[u8]) -> std::io::Result<Vec<u8>>,
{
    match transform(input) {
        Ok(data) => match BatchReportEvent::decode(&*data) {
            Ok(parsed) => {
                let dto = BatchReportEventDto::from_parsed(parsed, &data, input);
                Some(BatchReportEventParseResult {
                    parsed: Some(dto),
                    steps: vec![format!("步骤成功：{name}")],
                })
            }
            Err(_) => Some(BatchReportEventParseResult {
                parsed: None,
                steps: vec![format!("步骤失败：{name} (protobuf 解析失败)")],
            }),
        },
        Err(_) => Some(BatchReportEventParseResult {
            parsed: None,
            steps: vec![format!("步骤失败：{name} (处理错误)")],
        }),
    }
}

impl BatchReportEventDto {
    fn from_parsed(event: BatchReportEvent, decompressed: &[u8], original: &[u8]) -> Self {
        let mut json_value =
            serde_json::to_value(&event).unwrap_or_else(|_| json!({ "error": "序列化失败" }));
        strip_nulls(&mut json_value);

        let data_json = json_value;
        let decompressed_hex = hex_encode(decompressed);

        let input_md5 = format!("{:X}", md5::compute(original));

        let result_bytes = event.encode_to_vec();

        let result_md5 = format!("{:X}", md5::compute(&result_bytes));

        let is_match = input_md5.eq_ignore_ascii_case(&result_md5);

        BatchReportEventDto {
            data: data_json,
            decompressed_hex,
            input_md5,
            result_md5,
            is_match,
        }
    }
}

/// 实现 ServerHandler trait
#[tool_handler]
impl ServerHandler for ToolkitServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "kk_server_mcp_toolkit".to_string(),
                version: "0.1.0".to_string(),
                title: Some("KK 服务器工具包".to_string()),
                website_url: None,
                icons: None,
            },
            instructions: Some(
                "KK 服务器工具包 - 提供 BatchReportEvent 解析工具:\n\
                 • parse_batch_from_base64 - Base64 数据解析\n\
                 • parse_batch_from_hex - 十六进制字符串解析\n\
                 • parse_batch_from_file - 文件路径解析"
                    .to_string(),
            ),
        }
    }
}

/*
 * HTTP/SSE 支持 (TODO)
 *
 * 要添加 HTTP 支持，参考以下代码：
 *
 * 1. 在 Cargo.toml 中添加:
 *    rmcp-actix-web = { version = "0.8", features = ["transport-sse-server"] }
 *
 * 2. 使用以下代码:
 *    #[actix_web::main]
 *    async fn main() -> Result<(), Box<dyn std::error::Error>> {
 *        use rmcp_actix_web::transport::SseServer;
 *        let server = SseServer::serve("127.0.0.1:3000".parse()?).await?;
 *        let ct = server.with_service(|| ToolkitServer::new());
 *        println!("HTTP/SSE 服务器运行在: http://127.0.0.1:3000");
 *        ct.cancelled().await;
 *        Ok(())
 *    }
 */

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 注意：stdio 模式下不能使用 println!，因为会干扰 MCP 协议
    // 如需调试，使用 eprintln! 输出到 stderr
    eprintln!("🚀 启动 KK MCP 服务器 (stdio 模式)...");
    eprintln!("📦 提供 3 个解析工具");

    let service = ToolkitServer::new()
        .serve(stdio())
        .await?;

    eprintln!("✅ stdio 服务已启动，等待请求...");
    service.waiting().await?;

    Ok(())
}
