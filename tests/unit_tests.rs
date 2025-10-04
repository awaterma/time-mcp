use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use serde_json::{json, Value};
use std::time::{Duration, SystemTime};
use time_mcp_server::{
    auth::AuthManager,
    config::{ServerConfig, TransportType},
    models::{McpError, McpResponse, TokenInfo},
    tools::TimeTools,
};

#[cfg(test)]
mod time_tools_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_current_time_default_params() {
        let args = json!({});
        let result = TimeTools::get_current_time(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert!(response.get("timestamp").is_some());
        assert_eq!(response.get("timezone").unwrap().as_str().unwrap(), "UTC");
    }

    #[tokio::test]
    async fn test_get_current_time_with_timezone() {
        let args = json!({
            "timezone": "America/New_York",
            "format": "iso"
        });

        let result = TimeTools::get_current_time(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert!(response.get("timestamp").is_some());
        assert!(response.get("unix").is_some());
        assert_eq!(
            response.get("timezone").unwrap().as_str().unwrap(),
            "America/New_York"
        );
        assert!(response.get("formatted").is_some());
    }

    #[tokio::test]
    async fn test_get_current_time_unix_format() {
        let args = json!({
            "format": "unix",
            "timezone": "UTC"
        });

        let result = TimeTools::get_current_time(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert!(response.get("timestamp").is_some());
        assert!(response.get("timestamp").unwrap().is_i64());
        assert_eq!(response.get("timezone").unwrap().as_str().unwrap(), "UTC");
    }

    #[tokio::test]
    async fn test_get_current_time_custom_format() {
        let args = json!({
            "format": "custom",
            "custom_format": "%Y-%m-%d %H:%M:%S",
            "timezone": "UTC"
        });

        let result = TimeTools::get_current_time(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert!(response.get("formatted").is_some());
        let formatted = response.get("formatted").unwrap().as_str().unwrap();
        // Should match YYYY-MM-DD HH:MM:SS format
        assert!(formatted.len() == 19);
        assert!(formatted.chars().nth(4).unwrap() == '-');
        assert!(formatted.chars().nth(7).unwrap() == '-');
        assert!(formatted.chars().nth(10).unwrap() == ' ');
    }

    #[tokio::test]
    async fn test_get_current_time_human_format() {
        let args = json!({
            "format": "human",
            "timezone": "UTC"
        });

        let result = TimeTools::get_current_time(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert!(response.get("formatted").is_some());
    }

    #[tokio::test]
    async fn test_get_current_time_invalid_timezone() {
        let args = json!({
            "timezone": "Invalid/Timezone",
            "format": "iso"
        });

        let result = TimeTools::get_current_time(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid timezone"));
    }

    #[tokio::test]
    async fn test_get_current_time_invalid_format() {
        let args = json!({
            "format": "invalid_format",
            "timezone": "UTC"
        });

        let result = TimeTools::get_current_time(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid format"));
    }

    #[tokio::test]
    async fn test_get_current_time_missing_custom_format() {
        let args = json!({
            "format": "custom",
            "timezone": "UTC"
        });

        let result = TimeTools::get_current_time(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("custom_format required"));
    }

    #[tokio::test]
    async fn test_convert_timezone_valid() {
        let args = json!({
            "timestamp": "2023-01-01T12:00:00Z",
            "from_timezone": "UTC",
            "to_timezone": "America/New_York"
        });

        let result = TimeTools::convert_timezone(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert!(response.get("original").is_some());
        assert!(response.get("converted").is_some());

        let original = response.get("original").unwrap();
        assert_eq!(original.get("timezone").unwrap().as_str().unwrap(), "UTC");

        let converted = response.get("converted").unwrap();
        assert_eq!(
            converted.get("timezone").unwrap().as_str().unwrap(),
            "America/New_York"
        );
    }

    #[tokio::test]
    async fn test_convert_timezone_unix_timestamp() {
        let args = json!({
            "timestamp": "1672574400", // 2023-01-01 12:00:00 UTC
            "from_timezone": "UTC",
            "to_timezone": "Europe/London"
        });

        let result = TimeTools::convert_timezone(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_convert_timezone_missing_params() {
        let args = json!({
            "timestamp": "2023-01-01T12:00:00Z",
            "from_timezone": "UTC"
            // missing to_timezone
        });

        let result = TimeTools::convert_timezone(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("to_timezone required"));
    }

    #[tokio::test]
    async fn test_convert_timezone_invalid_timestamp() {
        let args = json!({
            "timestamp": "invalid-timestamp",
            "from_timezone": "UTC",
            "to_timezone": "America/New_York"
        });

        let result = TimeTools::convert_timezone(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_calculate_duration_seconds() {
        let args = json!({
            "start_time": "2023-01-01T10:00:00Z",
            "end_time": "2023-01-01T11:30:00Z",
            "units": "seconds"
        });

        let result = TimeTools::calculate_duration(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        let duration = response.get("duration").unwrap();
        assert_eq!(
            duration.get("total_seconds").unwrap().as_i64().unwrap(),
            5400
        );
        assert_eq!(duration.get("hours").unwrap().as_i64().unwrap(), 1);
        assert_eq!(duration.get("minutes").unwrap().as_i64().unwrap(), 90);
    }

    #[tokio::test]
    async fn test_calculate_duration_negative() {
        let args = json!({
            "start_time": "2023-01-01T11:00:00Z",
            "end_time": "2023-01-01T10:00:00Z",
            "units": "seconds"
        });

        let result = TimeTools::calculate_duration(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        let duration = response.get("duration").unwrap();
        assert_eq!(
            duration.get("total_seconds").unwrap().as_i64().unwrap(),
            -3600
        );
    }

    #[tokio::test]
    async fn test_calculate_duration_different_units() {
        let args = json!({
            "start_time": "2023-01-01T10:00:00Z",
            "end_time": "2023-01-01T12:00:00Z",
            "units": "hours"
        });

        let result = TimeTools::calculate_duration(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        let duration = response.get("duration").unwrap();
        assert_eq!(duration.get("hours").unwrap().as_i64().unwrap(), 2);
        assert!(duration
            .get("human_readable")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("2 hours"));
    }

    #[tokio::test]
    async fn test_calculate_duration_minutes_and_days() {
        let args = json!({
            "start_time": "2023-01-01T10:00:00Z",
            "end_time": "2023-01-03T10:00:00Z",
            "units": "minutes"
        });

        let result = TimeTools::calculate_duration(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        let duration = response.get("duration").unwrap();
        assert_eq!(duration.get("minutes").unwrap().as_i64().unwrap(), 2880);

        let args = json!({
            "start_time": "2023-01-01T10:00:00Z",
            "end_time": "2023-01-03T10:00:00Z",
            "units": "days"
        });

        let result = TimeTools::calculate_duration(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        let duration = response.get("duration").unwrap();
        assert_eq!(duration.get("days").unwrap().as_i64().unwrap(), 2);
    }
    #[tokio::test]
    async fn test_calculate_duration_invalid_units() {
        let args = json!({
            "start_time": "2023-01-01T10:00:00Z",
            "end_time": "2023-01-01T11:00:00Z",
            "units": "invalid_units"
        });

        let result = TimeTools::calculate_duration(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid units"));
    }

    #[tokio::test]
    async fn test_format_time_iso8601() {
        let args = json!({
            "timestamp": "2023-01-01T12:00:00Z",
            "format": "iso8601",
            "timezone": "UTC"
        });

        let result = TimeTools::format_time(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert!(response.get("formatted").is_some());
        let formatted = response.get("formatted").unwrap().as_str().unwrap();
        assert!(formatted.contains("2023-01-01T12:00:00"));
    }

    #[tokio::test]
    async fn test_format_time_rfc3339() {
        let args = json!({
            "timestamp": "2023-01-01T12:00:00Z",
            "format": "rfc3339",
            "timezone": "UTC"
        });

        let result = TimeTools::format_time(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert!(response.get("formatted").is_some());
        let formatted = response.get("formatted").unwrap().as_str().unwrap();
        assert!(formatted.contains("2023-01-01T12:00:00"));
    }

    #[tokio::test]
    async fn test_format_time_unix() {
        let args = json!({
            "timestamp": "2023-01-01T12:00:00Z",
            "format": "unix",
            "timezone": "UTC"
        });

        let result = TimeTools::format_time(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        let formatted = response.get("formatted").unwrap().as_str().unwrap();
        assert!(formatted.parse::<i64>().is_ok());
    }

    #[tokio::test]
    async fn test_format_time_custom() {
        let args = json!({
            "timestamp": "2023-01-01T12:00:00Z",
            "format": "custom",
            "custom_format": "%B %d, %Y",
            "timezone": "UTC"
        });

        let result = TimeTools::format_time(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        let formatted = response.get("formatted").unwrap().as_str().unwrap();
        assert_eq!(formatted, "January 01, 2023");
    }

    #[tokio::test]
    async fn test_get_timezone_info_utc() {
        let args = json!({
            "timezone": "UTC"
        });

        let result = TimeTools::get_timezone_info(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert_eq!(response.get("timezone").unwrap().as_str().unwrap(), "UTC");
        assert_eq!(response.get("offset").unwrap().as_str().unwrap(), "+00:00");
        assert_eq!(
            response.get("dst_active").unwrap().as_bool().unwrap(),
            false
        );
    }

    #[tokio::test]
    async fn test_get_timezone_info_with_dst() {
        let args = json!({
            "timezone": "America/New_York"
        });

        let result = TimeTools::get_timezone_info(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert_eq!(
            response.get("timezone").unwrap().as_str().unwrap(),
            "America/New_York"
        );
        assert!(response.get("offset").is_some());
        assert!(response.get("dst_active").is_some());
        assert!(response.get("abbreviation").is_some());
    }

    #[tokio::test]
    async fn test_list_timezones_all() {
        let args = json!({});

        let result = TimeTools::list_timezones(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        assert!(response.get("timezones").is_some());
        assert!(response.get("count").is_some());

        let timezones = response.get("timezones").unwrap().as_array().unwrap();
        assert!(timezones.len() > 0);
        assert!(response.get("count").unwrap().as_u64().unwrap() == timezones.len() as u64);
    }

    #[tokio::test]
    async fn test_list_timezones_filtered() {
        let args = json!({
            "region": "America"
        });

        let result = TimeTools::list_timezones(args).await;

        assert!(result.is_ok());
        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();

        let timezones = response.get("timezones").unwrap().as_array().unwrap();

        // All timezones should start with "America"
        for tz in timezones {
            let tz_str = tz.as_str().unwrap();
            assert!(tz_str.starts_with("America"));
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use clap::{Arg, Command};

    #[test]
    fn test_server_config_from_matches_stdio() {
        let app = Command::new("test")
            .disable_help_flag(true)
            .arg(Arg::new("transport").long("transport").required(true))
            .arg(Arg::new("host").long("host"))
            .arg(Arg::new("port").long("port"));

        let matches = app
            .try_get_matches_from(vec!["test", "--transport", "stdio"])
            .unwrap();

        let config = ServerConfig::from_matches(&matches).unwrap();

        match config.transport {
            TransportType::Stdio => {}
            _ => panic!("Expected Stdio transport"),
        }

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_server_config_from_matches_http() {
        let app = Command::new("test")
            .disable_help_flag(true)
            .arg(Arg::new("transport").long("transport").required(true))
            .arg(Arg::new("host").long("host"))
            .arg(Arg::new("port").long("port"));

        let matches = app
            .try_get_matches_from(vec![
                "test",
                "--transport",
                "http",
                "--host",
                "0.0.0.0",
                "--port",
                "3000",
            ])
            .unwrap();

        let config = ServerConfig::from_matches(&matches).unwrap();

        match config.transport {
            TransportType::Http { host, port } => {
                assert_eq!(host, "0.0.0.0");
                assert_eq!(port, 3000);
            }
            _ => panic!("Expected HTTP transport"),
        }

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn test_server_config_invalid_transport() {
        let app = Command::new("test")
            .disable_help_flag(true)
            .arg(Arg::new("transport").long("transport").required(true))
            .arg(Arg::new("host").long("host"))
            .arg(Arg::new("port").long("port"));

        let matches = app
            .try_get_matches_from(vec!["test", "--transport", "invalid"])
            .unwrap();

        let result = ServerConfig::from_matches(&matches);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid transport type"));
    }

    #[test]
    fn test_server_config_auth_enabled() {
        std::env::set_var("OAUTH_ENABLED", "true");

        let app = Command::new("test")
            .disable_help_flag(true)
            .arg(Arg::new("transport").long("transport").required(true))
            .arg(Arg::new("host").long("host"))
            .arg(Arg::new("port").long("port"));

        let matches = app
            .try_get_matches_from(vec!["test", "--transport", "stdio"])
            .unwrap();

        let config = ServerConfig::from_matches(&matches).unwrap();
        assert!(config.auth_enabled);

        // Clean up
        std::env::remove_var("OAUTH_ENABLED");
    }
}

#[cfg(test)]
mod models_tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_token_info_not_expired() {
        let future_time = SystemTime::now() + Duration::from_secs(3600);
        let token = TokenInfo {
            user_id: "test_user".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            expires_at: future_time,
        };

        assert!(!token.is_expired());
    }

    #[test]
    fn test_token_info_expired() {
        let past_time = SystemTime::now() - Duration::from_secs(3600);
        let token = TokenInfo {
            user_id: "test_user".to_string(),
            scopes: vec!["read".to_string()],
            expires_at: past_time,
        };

        assert!(token.is_expired());
    }

    #[test]
    fn test_mcp_error_creation() {
        let error = McpError::new(-32602, "Invalid parameters");
        assert_eq!(error.code, -32602);
        assert_eq!(error.message, "Invalid parameters");
    }

    #[test]
    fn test_mcp_error_invalid_params() {
        let error = McpError::invalid_params("Missing required field");
        assert_eq!(error.code, -32602);
        assert_eq!(error.message, "Missing required field");
    }

    #[test]
    fn test_mcp_error_method_not_found() {
        let error = McpError::method_not_found("Unknown method");
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Unknown method");
    }

    #[test]
    fn test_mcp_error_internal_error() {
        let error = McpError::internal_error("Server error");
        assert_eq!(error.code, -32603);
        assert_eq!(error.message, "Server error");
    }

    #[test]
    fn test_mcp_response_success() {
        let id = json!(1);
        let result = json!({"message": "success"});
        let response = McpResponse::success(id.clone(), result.clone());

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, id);
        assert_eq!(response.result.unwrap(), result);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_mcp_response_error() {
        let id = json!(1);
        let error = McpError::invalid_params("Test error");
        let response = McpResponse::<()>::error(id.clone(), error);

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, id);
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let err = response.error.unwrap();
        assert_eq!(err.code, -32602);
        assert_eq!(err.message, "Test error");
    }

    #[test]
    fn test_mcp_error_from_anyhow() {
        let anyhow_error = anyhow::anyhow!("Test anyhow error");
        let mcp_error: McpError = anyhow_error.into();

        assert_eq!(mcp_error.code, -32603);
        assert_eq!(mcp_error.message, "Test anyhow error");
    }

    #[test]
    fn test_mcp_error_serialization() {
        let error = McpError::new(-32602, "Test message");
        let serialized = serde_json::to_string(&error).unwrap();
        let expected = r#"{"code":-32602,"message":"Test message"}"#;

        assert_eq!(serialized, expected);
    }
}

#[cfg(test)]
mod edge_cases_tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_json_arguments() {
        let args = json!(null);
        let result = TimeTools::get_current_time(args).await;

        // Should handle null arguments gracefully
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_malformed_timestamp_formats() {
        let invalid_timestamps = vec![
            "not-a-date",
            "2023/13/45",
            "",
            "2023-01-01", // Missing time component
            "12:00:00",   // Missing date component
        ];

        for timestamp in invalid_timestamps {
            let args = json!({
                "timestamp": timestamp,
                "from_timezone": "UTC",
                "to_timezone": "America/New_York"
            });

            let result = TimeTools::convert_timezone(args).await;
            assert!(result.is_err(), "Should fail for timestamp: {}", timestamp);
        }
    }

    #[tokio::test]
    async fn test_extreme_unix_timestamps() {
        // Test very early timestamp (1970)
        let args = json!({
            "timestamp": "0",
            "from_timezone": "UTC",
            "to_timezone": "America/New_York"
        });
        let result = TimeTools::convert_timezone(args).await;
        assert!(result.is_ok());

        // Test far future timestamp (year 2038+)
        let args = json!({
            "timestamp": "2147483647",  // Max 32-bit Unix timestamp
            "from_timezone": "UTC",
            "to_timezone": "America/New_York"
        });
        let result = TimeTools::convert_timezone(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_timezone_edge_cases() {
        let edge_case_timezones = vec![
            "UTC",
            "GMT",
            "Etc/GMT+12",
            "Etc/GMT-12",
            "Pacific/Kiritimati", // UTC+14
            "Pacific/Midway",     // UTC-11
        ];

        for tz in edge_case_timezones {
            let args = json!({
                "timezone": tz
            });

            let result = TimeTools::get_timezone_info(args).await;
            assert!(result.is_ok(), "Should work for timezone: {}", tz);
        }
    }

    #[tokio::test]
    async fn test_leap_year_handling() {
        let args = json!({
            "timestamp": "2024-02-29T12:00:00Z",  // Leap year date
            "from_timezone": "UTC",
            "to_timezone": "Europe/London"
        });

        let result = TimeTools::convert_timezone(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dst_transitions() {
        // Test DST transition dates for US Eastern Time
        let spring_forward = json!({
            "timestamp": "2024-03-10T07:00:00Z",  // 2 AM EST becomes 3 AM EDT
            "from_timezone": "UTC",
            "to_timezone": "America/New_York"
        });

        let result = TimeTools::convert_timezone(spring_forward).await;
        assert!(result.is_ok());

        let fall_back = json!({
            "timestamp": "2024-11-03T06:00:00Z",  // 2 AM EDT becomes 1 AM EST
            "from_timezone": "UTC",
            "to_timezone": "America/New_York"
        });

        let result = TimeTools::convert_timezone(fall_back).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_duration_calculation_edge_cases() {
        // Same timestamp for start and end
        let args = json!({
            "start_time": "2023-01-01T12:00:00Z",
            "end_time": "2023-01-01T12:00:00Z",
            "units": "seconds"
        });

        let result = TimeTools::calculate_duration(args).await;
        assert!(result.is_ok());

        let response_str = result.unwrap();
        let response: Value = serde_json::from_str(&response_str).unwrap();
        let duration = response.get("duration").unwrap();
        assert_eq!(duration.get("total_seconds").unwrap().as_i64().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_custom_format_edge_cases() {
        // Test valid custom format
        let args = json!({
            "format": "custom",
            "custom_format": "%Y-%m-%d",
            "timezone": "UTC"
        });

        let result = TimeTools::get_current_time(args).await;
        assert!(result.is_ok(), "Valid format should work");

        // Test escaped percent
        let args = json!({
            "format": "custom",
            "custom_format": "%%",
            "timezone": "UTC"
        });

        let result = TimeTools::get_current_time(args).await;
        assert!(result.is_ok(), "Escaped percent should work");

        // Test missing custom_format field
        let args = json!({
            "format": "custom",
            "timezone": "UTC"
        });

        let result = TimeTools::get_current_time(args).await;
        assert!(result.is_err(), "Missing custom_format should fail");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("custom_format required"));
    }
}

#[cfg(test)]
mod auth_tests {
    use super::*;

    async fn add_token(auth: &AuthManager, token: String, info: TokenInfo) {
        let mut tokens = auth.tokens.write().await;
        tokens.insert(token, info);
    }

    async fn remove_expired_tokens(auth: &AuthManager) {
        let mut tokens = auth.tokens.write().await;
        tokens.retain(|_, info| !info.is_expired());
    }

    #[tokio::test]
    async fn test_auth_manager_new() {
        let auth_enabled = AuthManager::new(true);
        let auth_disabled = AuthManager::new(false);

        let headers = HeaderMap::new();

        // Disabled auth should always pass
        assert!(auth_disabled.authenticate(&headers).await.is_ok());

        // Enabled auth should require authorization header
        assert!(auth_enabled.authenticate(&headers).await.is_err());
    }

    #[tokio::test]
    async fn test_auth_disabled() {
        let auth = AuthManager::new(false);
        let empty_headers = HeaderMap::new();

        // Should always pass when auth is disabled
        assert!(auth.authenticate(&empty_headers).await.is_ok());
    }

    #[tokio::test]
    async fn test_auth_missing_authorization_header() {
        let auth = AuthManager::new(true);
        let empty_headers = HeaderMap::new();

        let result = auth.authenticate(&empty_headers).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, 401);
        assert_eq!(error.message, "Authorization header required");
    }

    #[tokio::test]
    async fn test_auth_invalid_authorization_format() {
        let auth = AuthManager::new(true);
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("Basic invalid"),
        );

        let result = auth.authenticate(&headers).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, 401);
        assert_eq!(error.message, "Invalid authorization format");
    }

    #[tokio::test]
    async fn test_auth_invalid_token() {
        let auth = AuthManager::new(true);
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("Bearer invalid_token"),
        );

        let result = auth.authenticate(&headers).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, 401);
        assert_eq!(error.message, "Invalid token");
    }

    #[tokio::test]
    async fn test_auth_valid_token() {
        let auth = AuthManager::new(true);

        // Add a valid token
        let future_time = SystemTime::now() + Duration::from_secs(3600);
        let token_info = TokenInfo {
            user_id: "test_user".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            expires_at: future_time,
        };

        add_token(&auth, "valid_token".to_string(), token_info).await;

        // Test with valid token
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("Bearer valid_token"),
        );

        let result = auth.authenticate(&headers).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_auth_expired_token() {
        let auth = AuthManager::new(true);

        // Add an expired token
        let past_time = SystemTime::now() - Duration::from_secs(3600);
        let token_info = TokenInfo {
            user_id: "test_user".to_string(),
            scopes: vec!["read".to_string()],
            expires_at: past_time,
        };

        add_token(&auth, "expired_token".to_string(), token_info).await;

        // Test with expired token
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("Bearer expired_token"),
        );

        let result = auth.authenticate(&headers).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, 401);
        assert_eq!(error.message, "Token expired");
    }

    #[tokio::test]
    async fn test_remove_expired_tokens() {
        let auth = AuthManager::new(true);

        // Add both valid and expired tokens
        let future_time = SystemTime::now() + Duration::from_secs(3600);
        let past_time = SystemTime::now() - Duration::from_secs(3600);

        let valid_token_info = TokenInfo {
            user_id: "valid_user".to_string(),
            scopes: vec!["read".to_string()],
            expires_at: future_time,
        };

        let expired_token_info = TokenInfo {
            user_id: "expired_user".to_string(),
            scopes: vec!["read".to_string()],
            expires_at: past_time,
        };

        add_token(&auth, "valid_token".to_string(), valid_token_info).await;
        add_token(&auth, "expired_token".to_string(), expired_token_info).await;

        // Remove expired tokens
        remove_expired_tokens(&auth).await;

        // Test that valid token still works
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("Bearer valid_token"),
        );
        assert!(auth.authenticate(&headers).await.is_ok());

        // Test that expired token was removed
        let mut expired_headers = HeaderMap::new();
        expired_headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("Bearer expired_token"),
        );
        let result = auth.authenticate(&expired_headers).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().message, "Invalid token");
    }

    #[test]
    fn test_mcp_error_to_status_code() {
        let error_401 = McpError::new(401, "Unauthorized");
        assert_eq!(StatusCode::from(error_401), StatusCode::UNAUTHORIZED);

        let error_400 = McpError::new(400, "Bad Request");
        assert_eq!(StatusCode::from(error_400), StatusCode::BAD_REQUEST);

        let error_32602 = McpError::new(-32602, "Invalid params");
        assert_eq!(StatusCode::from(error_32602), StatusCode::BAD_REQUEST);

        let error_404 = McpError::new(404, "Not Found");
        assert_eq!(StatusCode::from(error_404), StatusCode::NOT_FOUND);

        let error_32601 = McpError::new(-32601, "Method not found");
        assert_eq!(StatusCode::from(error_32601), StatusCode::NOT_FOUND);

        let error_500 = McpError::new(500, "Internal Error");
        assert_eq!(
            StatusCode::from(error_500),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        let error_unknown = McpError::new(999, "Unknown");
        assert_eq!(
            StatusCode::from(error_unknown),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
