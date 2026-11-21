pub mod deepseek;
pub mod service;
pub mod sse;
#[cfg(test)]
mod tests;
pub mod video_cloud;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatFailure {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}
impl std::fmt::Display for ChatFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ChatFailure {}
impl ChatFailure {
    pub fn new(code: &str) -> Self {
        let (message, retryable) = match code {
            "CHAT_IMAGE_INVALID" => ("图片无法保存，请重新选择常用格式的图片。", false),
            "CHAT_IMAGE_LIMIT" => ("本机聊天图片数量达到上限。这条消息尚未发送。", false),
            "KEY_NOT_CONFIGURED" => ("请先在设置中配置本机 DeepSeek API Key。", false),
            "KEY_FORMAT_INVALID" => ("密钥格式不正确，请检查是否含空格或换行。", false),
            "SECURE_STORAGE_UNSUPPORTED" => {
                ("当前平台尚未配置安全凭据存储；不会使用明文回退。", false)
            }
            "SECURE_STORAGE_CORRUPT" => ("本机凭据无法解密，请删除后重新配置。", false),
            "SECURE_STORAGE_UNAVAILABLE" => ("系统安全凭据存储不可用。", false),
            "HTTP_401" => ("DeepSeek 身份验证失败，请检查密钥。", false),
            "HTTP_402" => ("DeepSeek 账户额度不足。", false),
            "HTTP_429" => ("请求过于频繁，请稍后重试。", true),
            "HTTP_403" => ("当前密钥没有访问此服务的权限。", false),
            "HTTP_400" => ("DeepSeek 拒绝了请求参数，请检查模型和聊天设置。", false),
            "HTTP_SERVER" => ("DeepSeek 服务暂时不可用，请稍后手动重试。", true),
            "NETWORK_ERROR" => ("网络连接失败，消息已保留。请检查网络后重试。", true),
            "TIMEOUT" => ("生成超时，已保留收到的内容。", true),
            "STREAM_INCOMPLETE" => ("回复流意外中断，已保留收到的内容。", true),
            "STREAM_INVALID" => ("服务返回了无法识别的流式数据。", true),
            "OUTPUT_LIMIT" => ("回复达到本地长度限制，已保留当前内容。", false),
            "PET_PERSONA_CHANGED" => (
                "当前人格已切换。请核对桌宠上的名字后再发送，草稿已保留。",
                false,
            ),
            "PET_WINDOW_ERROR" => ("桌宠窗口操作失败，请从托盘找回桌宠或重新打开应用。", true),
            "PET_STARTUP_ERROR" => (
                "Windows 登录启动设置未保存。请检查本机当前账户的权限。",
                true,
            ),
            "CANCELLED" => ("生成已停止。", false),
            "PERSONA_PROTECTED" => ("默认人格和最后一个可用人格受到保护。", false),
            "PERSONA_RECYCLE_CHANGED" => ("回收站内容已变化，请刷新并重新确认。", false),
            "PERSONA_TRASHED" => ("人格已移入回收站，生成已结束。", false),
            "PLUGIN_INVALID" => ("插件清单格式、版本、路径或权限不受支持。", false),
            "PLUGIN_CHECKSUM" => ("插件清单的 SHA-256 与内容不符。", false),
            "PLUGIN_CHANGED" => ("请检查插件版本、校验值和当前平台后重新操作。", false),
            "PLUGIN_CONFIG" => ("插件配置无效或已达到本机数量上限。", false),
            "PLUGIN_PERMISSION" => ("请启用插件并确认需要的权限。", false),
            "PLUGIN_PERSONA_CHANGED" => ("当前人格已切换。请核对人格后再操作。", false),
            "BUSY" => ("已有回复正在生成，请先停止或等待完成。", false),
            "DATA_VERSION_CONFLICT" => (
                "这条数据已更新。请刷新后核对；当前内容和待确认修订均已保留。",
                false,
            ),
            "APP_RESTARTED" => ("上次生成因应用退出而中断。", true),
            "STORAGE_ERROR" => ("本地数据库写入失败，请检查存储空间。", false),
            "DATABASE_IN_USE" => ("本地档案已由另一个应用实例打开，请先关闭该实例。", false),
            "CONTEXT_BUDGET_EXCEEDED" => (
                "本次输入与人设超过上下文预算。消息已保留，请缩短输入或在陪伴设置提高预算。",
                false,
            ),
            _ => ("操作未能完成，请检查输入或稍后重试。", false),
        };
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }
}
impl From<companion_secrets::VaultError> for ChatFailure {
    fn from(e: companion_secrets::VaultError) -> Self {
        Self::new(&e.to_string())
    }
}
impl From<companion_core::Error> for ChatFailure {
    fn from(e: companion_core::Error) -> Self {
        match e {
            companion_core::Error::Invalid("CONTEXT_BUDGET_EXCEEDED") => {
                Self::new("CONTEXT_BUDGET_EXCEEDED")
            }
            companion_core::Error::Invalid(
                code @ ("PERSONA_PROTECTED"
                | "PERSONA_RECYCLE_CHANGED"
                | "CHAT_IMAGE_INVALID"
                | "CHAT_IMAGE_LIMIT"),
            ) => Self::new(code),
            companion_core::Error::Invalid(
                code @ ("PLUGIN_INVALID"
                | "PLUGIN_CHECKSUM"
                | "PLUGIN_CHANGED"
                | "PLUGIN_CONFIG"
                | "PLUGIN_PERMISSION"
                | "PLUGIN_PERSONA_CHANGED"),
            ) => Self::new(code),
            companion_core::Error::Invalid(_) | companion_core::Error::Conflict => {
                Self::new("INVALID_INPUT")
            }
            companion_core::Error::DatabaseInUse => Self::new("DATABASE_IN_USE"),
            _ => Self::new("STORAGE_ERROR"),
        }
    }
}
