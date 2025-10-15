# KK Server MCP Toolkit

一个使用 Rust 和 rmcp 构建的 Model Context Protocol (MCP) 服务器工具包。

## 功能特性

这个 MCP 服务器提供了 **3 个 BatchReportEvent 解析工具**：

### 🔧 解析工具
- **parse_batch_from_base64** - 解析 Base64 编码的 BatchReportEvent (gzip 压缩)
- **parse_batch_from_hex** - 解析十六进制字符串的 BatchReportEvent (gzip 压缩)
- **parse_batch_from_file** - 从文件路径解析 BatchReportEvent (gzip 压缩)

## 快速开始

### 方式 1：下载预编译版本（推荐）

从 [Releases](../../releases) 页面下载对应平台的版本：

- **macOS (Apple Silicon)**: `kk_server_mcp_toolkit-macos-arm64.tar.gz`
- **macOS (Intel)**: `kk_server_mcp_toolkit-macos-x86_64.tar.gz`
- **Linux**: `kk_server_mcp_toolkit-linux-x86_64.tar.gz`
- **Windows**: `kk_server_mcp_toolkit-windows-x86_64.zip`

解压后：
```bash
# macOS/Linux
tar -xzf kk_server_mcp_toolkit-*.tar.gz
chmod +x kk_server_mcp_toolkit

# macOS 需要移除隔离标记
xattr -d com.apple.quarantine kk_server_mcp_toolkit
```

### 方式 2：从源码编译

```bash
# 克隆仓库
git clone <your-repo-url>
cd kk_server_mcp_toolkit

# 编译
cargo build --release

# 二进制文件位于
./target/release/kk_server_mcp_toolkit
```

## 配置 LM Studio

在 `~/.lmstudio/mcp.json` 中添加：

```json
{
  "mcpServers": {
    "kk-toolkit": {
      "command": "/path/to/kk_server_mcp_toolkit"
    }
  }
}
```

## 配置 Claude Desktop

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "kk-toolkit": {
      "command": "/path/to/kk_server_mcp_toolkit"
    }
  }
}
```

## 技术栈

- **Rust 2021 Edition**
- **rmcp 0.8** - 官方 Rust MCP SDK
- **Tokio** - 异步运行时
- **Anyhow** - 错误处理
- **Schemars** - JSON Schema 生成

## 项目结构

```
kk_server_mcp_toolkit/
├── Cargo.toml          # 项目配置和依赖
├── Cargo.lock          # 依赖锁定文件
├── src/
│   └── main.rs         # MCP 服务器实现 (199 行)
├── target/
│   └── release/        # 编译后的二进制文件
│       └── kk_server_mcp_toolkit
└── README.md           # 本文件
```

## 代码示例

### 定义一个简单的 Tool

```rust
#[tool(description = "回显消息到客户端")]
async fn echo(&self, message: String) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![Content::text(format!(
        "Echo: {}", message
    ))]))
}
```

### 带参数的 Tool

```rust
#[tool(description = "计算两个数字的和")]
async fn add(&self, a: f64, b: f64) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![Content::text(format!(
        "{} + {} = {}", a, b, a + b
    ))]))
}
```

### 实现 ServerHandler

```rust
impl ServerHandler for ToolkitServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "kk_server_mcp_toolkit".to_string(),
                version: "0.1.0".to_string(),
                title: Some("KK 服务器工具包".to_string()),
                // ...
            },
            // ...
        }
    }
}
```

## 开发

### 添加新工具

1. 在 `ToolkitServer` impl 块中添加新方法
2. 使用 `#[tool(description = "...")]` 标注
3. 方法签名：`async fn tool_name(&self, params...) -> Result<CallToolResult, ErrorData>`
4. 使用 `CallToolResult::success(vec![Content::text(...)])` 返回结果
5. 重新编译：`cargo build --release`

### 运行测试

```bash
cargo test
```

### 代码检查

```bash
cargo clippy
```

### 格式化代码

```bash
cargo fmt
```

## 性能

- **编译时间**: ~16 秒 (release)
- **二进制大小**: 约 4-5 MB (release, stripped)
- **内存占用**: 最小
- **启动时间**: <100ms

## 特性

### 已实现 ✅
- [x] stdio 传输
- [x] 11 个实用工具
- [x] 异步支持 (Tokio)
- [x] 类型安全
- [x] 错误处理
- [x] 计数器状态管理

### 计划中 🚧
- [ ] HTTP/SSE 传输
- [ ] 更多工具（文件操作、网络请求等）
- [ ] 配置文件支持
- [ ] 日志记录
- [ ] 性能监控

## 许可证

MIT License

## 相关资源

- [MCP 官方文档](https://modelcontextprotocol.io/)
- [rmcp Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [MCP 规范](https://modelcontextprotocol.io/specification/)
- [Rust 异步编程](https://rust-lang.github.io/async-book/)

## 技术说明

### Protobuf 代码生成

本项目使用预生成的 Protobuf Rust 代码（位于 `src/generated/`），**无需**在构建时依赖 `protoc` 编译器。

**优点：**
- ✅ 无需安装 Protocol Buffers 编译器
- ✅ 构建更快、更简单
- ✅ 更容易在 CI/CD 中使用
- ✅ 分发和编译更简单

