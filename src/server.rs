use std::sync::Arc;
use std::borrow::Cow;
use jetkvm_client::jetkvm_rpc_client::{JetKvmRpcClient, SignalingMethod};
use rmcp::{
    model::{CallToolResult, Content, ErrorCode, ServerCapabilities, ServerInfo},
    tool, tool_router, tool_handler, ErrorData as McpError, ServerHandler,
    handler::server::tool::ToolRouter,
};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct JetKvmServer {
    client: Arc<Mutex<Option<JetKvmRpcClient>>>,
    host: String,
    password: String,
    api: String,
    no_auto_logout: bool,
    pub tool_router: ToolRouter<Self>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SendTextParams {
    text: String,
}

#[derive(Debug, Clone, Copy)]
struct Resolution {
    width: u32,
    height: u32,
}

const XGA: Resolution = Resolution { width: 1024, height: 768 };      // 4:3
const WXGA: Resolution = Resolution { width: 1280, height: 800 };     // 16:10
const FWXGA: Resolution = Resolution { width: 1366, height: 768 };    // ~16:9

fn select_target_resolution(original_width: u32, original_height: u32) -> Resolution {
    let aspect_ratio = original_width as f64 / original_height as f64;
    
    let ratio_4_3 = 4.0 / 3.0;
    let ratio_16_10 = 16.0 / 10.0;
    let ratio_16_9 = 16.0 / 9.0;
    
    let diff_4_3 = (aspect_ratio - ratio_4_3).abs();
    let diff_16_10 = (aspect_ratio - ratio_16_10).abs();
    let diff_16_9 = (aspect_ratio - ratio_16_9).abs();
    
    if diff_4_3 <= diff_16_10 && diff_4_3 <= diff_16_9 {
        XGA
    } else if diff_16_10 <= diff_16_9 {
        WXGA
    } else {
        FWXGA
    }
}

#[tool_router]
impl JetKvmServer {
    pub fn new(host: String, password: String) -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
            host,
            password,
            api: "/webrtc/session".to_string(),
            no_auto_logout: false,
            tool_router: Self::tool_router(),
        }
    }

    pub async fn connect(&self) -> Result<(), String> {
        let mut client_guard = self.client.lock().await;
        let mut client = JetKvmRpcClient::new(
            self.host.clone(),
            self.password.clone(),
            self.api.clone(),
            self.no_auto_logout,
            SignalingMethod::Auto,
        );
        
        client.connect().await.map_err(|e| format!("Failed to connect: {}", e))?;
        client.wait_for_channel_open().await.map_err(|e| format!("Failed to open channel: {}", e))?;
        
        *client_guard = Some(client);
        Ok(())
    }

    #[tool(description = "Send text to the remote system")]
    async fn send_text(&self, params: rmcp::handler::server::wrapper::Parameters<SendTextParams>) -> Result<CallToolResult, McpError> {
        let client_guard = self.client.lock().await;
        let client = client_guard.as_ref().ok_or_else(|| McpError {
            code: ErrorCode(-32001),
            message: Cow::Borrowed("Not connected to JetKVM device"),
            data: None,
        })?;

        jetkvm_client::keyboard::rpc_sendtext(client, &params.0.text)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32002),
                message: Cow::Owned(format!("Failed to send text: {}", e)),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Successfully sent text: {}",
            params.0.text
        ))]))
    }

    #[tool(description = "Capture a screenshot from the remote system's display. Automatically scaled to standard resolutions (XGA 1024x768 for 4:3, WXGA 1280x800 for 16:10, or FWXGA 1366x768 for 16:9) based on aspect ratio. Aspect ratio always preserved.")]
    async fn screenshot(&self) -> Result<CallToolResult, McpError> {
        use image::ImageReader;
        use std::io::Cursor;

        let client_guard = self.client.lock().await;
        let client = client_guard.as_ref().ok_or_else(|| McpError {
            code: ErrorCode(-32001),
            message: Cow::Borrowed("Not connected to JetKVM device"),
            data: None,
        })?;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let png_data = client.video_capture.capture_screenshot_png()
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32003),
                message: Cow::Owned(format!("Failed to capture screenshot: {}", e)),
                data: None,
            })?;

        let img = ImageReader::new(Cursor::new(&png_data))
            .with_guessed_format()
            .map_err(|e| McpError {
                code: ErrorCode(-32004),
                message: Cow::Owned(format!("Failed to decode image: {}", e)),
                data: None,
            })?
            .decode()
            .map_err(|e| McpError {
                code: ErrorCode(-32004),
                message: Cow::Owned(format!("Failed to decode image: {}", e)),
                data: None,
            })?;

        let (original_width, original_height) = (img.width(), img.height());
        let target = select_target_resolution(original_width, original_height);

        let (new_width, new_height) = if original_width > target.width || original_height > target.height {
            let width_ratio = target.width as f64 / original_width as f64;
            let height_ratio = target.height as f64 / original_height as f64;
            let scale = width_ratio.min(height_ratio);
            
            (
                (original_width as f64 * scale) as u32,
                (original_height as f64 * scale) as u32,
            )
        } else {
            (original_width, original_height)
        };

        let final_png_data = if new_width != original_width || new_height != original_height {
            let resized = image::imageops::resize(
                &img,
                new_width,
                new_height,
                image::imageops::FilterType::Lanczos3,
            );

            let mut output = Vec::new();
            image::DynamicImage::ImageRgba8(resized)
                .write_to(&mut Cursor::new(&mut output), image::ImageFormat::Png)
                .map_err(|e| McpError {
                    code: ErrorCode(-32005),
                    message: Cow::Owned(format!("Failed to encode resized image: {}", e)),
                    data: None,
                })?;
            output
        } else {
            png_data
        };

        let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &final_png_data);
        
        Ok(CallToolResult::success(vec![
            Content::image(base64_data, "image/png"),
        ]))
    }
}

#[tool_handler]
impl ServerHandler for JetKvmServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("JetKVM MCP Server - Control your JetKVM device remotely. Send text, control mouse, capture screenshots, and more.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
