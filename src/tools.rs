use anyhow::Result;
use chrono::{DateTime, Utc, TimeZone, Offset};
use chrono_tz::{Tz, TZ_VARIANTS};
use serde_json::{json, Value};

pub struct TimeTools;

impl TimeTools {
    pub async fn get_current_time(arguments: Value) -> Result<String> {
        let timezone = arguments.get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("UTC");
        let format = arguments.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("iso");

        let tz: Tz = timezone.parse()
            .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", timezone))?;
        
        let now_utc = Utc::now();
        let now_tz = now_utc.with_timezone(&tz);

        let result = match format {
            "iso" => json!({
                "timestamp": now_tz.to_rfc3339(),
                "unix": now_utc.timestamp(),
                "timezone": timezone,
                "formatted": now_tz.format("%A, %B %d, %Y at %I:%M %p %Z").to_string()
            }),
            "unix" => json!({
                "timestamp": now_utc.timestamp(),
                "timezone": timezone
            }),
            "human" => json!({
                "formatted": now_tz.format("%A, %B %d, %Y at %I:%M %p %Z").to_string(),
                "timezone": timezone
            }),
            "custom" => {
                let custom_format = arguments.get("custom_format")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("custom_format required when format is 'custom'"))?;
                json!({
                    "formatted": now_tz.format(custom_format).to_string(),
                    "timezone": timezone
                })
            },
            _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
        };

        Ok(result.to_string())
    }

    pub async fn convert_timezone(arguments: Value) -> Result<String> {
        let timestamp_str = arguments.get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("timestamp required"))?;
        let from_tz_str = arguments.get("from_timezone")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("from_timezone required"))?;
        let to_tz_str = arguments.get("to_timezone")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("to_timezone required"))?;

        let from_tz: Tz = from_tz_str.parse()
            .map_err(|_| anyhow::anyhow!("Invalid from_timezone: {}", from_tz_str))?;
        let to_tz: Tz = to_tz_str.parse()
            .map_err(|_| anyhow::anyhow!("Invalid to_timezone: {}", to_tz_str))?;

        let dt = Self::parse_timestamp(timestamp_str)?
            .with_timezone(&from_tz);
        let converted = dt.with_timezone(&to_tz);

        let result = json!({
            "original": {
                "timestamp": dt.to_rfc3339(),
                "timezone": from_tz_str
            },
            "converted": {
                "timestamp": converted.to_rfc3339(),
                "timezone": to_tz_str
            }
        });

        Ok(result.to_string())
    }

    pub async fn calculate_duration(arguments: Value) -> Result<String> {
        let start_str = arguments.get("start_time")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("start_time required"))?;
        let end_str = arguments.get("end_time")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("end_time required"))?;
        let units = arguments.get("units")
            .and_then(|v| v.as_str())
            .unwrap_or("seconds");

        let start_dt = Self::parse_timestamp(start_str)?;
        let end_dt = Self::parse_timestamp(end_str)?;
        
        let duration = end_dt.signed_duration_since(start_dt);
        let total_seconds = duration.num_seconds();
        
        let result = match units {
            "seconds" => json!({
                "duration": {
                    "total_seconds": total_seconds,
                    "hours": total_seconds / 3600,
                    "minutes": total_seconds / 60,
                    "human_readable": format!("{} seconds", total_seconds)
                }
            }),
            "minutes" => {
                let minutes = total_seconds / 60;
                json!({
                    "duration": {
                        "total_seconds": total_seconds,
                        "minutes": minutes,
                        "hours": minutes / 60,
                        "human_readable": format!("{} minutes", minutes)
                    }
                })
            },
            "hours" => {
                let hours = total_seconds / 3600;
                json!({
                    "duration": {
                        "total_seconds": total_seconds,
                        "hours": hours,
                        "minutes": total_seconds / 60,
                        "human_readable": format!("{} hours", hours)
                    }
                })
            },
            "days" => {
                let days = total_seconds / (24 * 3600);
                json!({
                    "duration": {
                        "total_seconds": total_seconds,
                        "days": days,
                        "hours": total_seconds / 3600,
                        "human_readable": format!("{} days", days)
                    }
                })
            },
            _ => return Err(anyhow::anyhow!("Invalid units: {}", units)),
        };

        Ok(result.to_string())
    }

    pub async fn format_time(arguments: Value) -> Result<String> {
        let timestamp_str = arguments.get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("timestamp required"))?;
        let format = arguments.get("format")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("format required"))?;
        let timezone_str = arguments.get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("UTC");

        let dt = Self::parse_timestamp(timestamp_str)?;
        let tz: Tz = timezone_str.parse()
            .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", timezone_str))?;
        let dt_tz = dt.with_timezone(&tz);

        let result = match format {
            "iso8601" | "rfc3339" => json!({
                "formatted": dt_tz.to_rfc3339(),
                "timezone": timezone_str
            }),
            "unix" => json!({
                "formatted": dt.timestamp().to_string(),
                "timezone": timezone_str
            }),
            "custom" => {
                let custom_format = arguments.get("custom_format")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("custom_format required when format is 'custom'"))?;
                json!({
                    "formatted": dt_tz.format(custom_format).to_string(),
                    "timezone": timezone_str
                })
            },
            _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
        };

        Ok(result.to_string())
    }

    pub async fn get_timezone_info(arguments: Value) -> Result<String> {
        let timezone_str = arguments.get("timezone")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("timezone required"))?;

        let tz: Tz = timezone_str.parse()
            .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", timezone_str))?;
        
        let now = Utc::now().with_timezone(&tz);
        let offset = now.offset();
        
        let offset_seconds = offset.fix().local_minus_utc();
        let dst_active = offset_seconds != tz.offset_from_utc_datetime(&now.naive_utc()).fix().local_minus_utc();
        let abbreviation = format!("{}", now.format("%Z"));
        
        let offset_hours = offset_seconds / 3600;
        let offset_minutes = (offset_seconds % 3600) / 60;
        let offset_str = format!("{:+03}:{:02}", offset_hours, offset_minutes.abs());

        let result = json!({
            "timezone": timezone_str,
            "offset": offset_str,
            "dst_active": dst_active,
            "abbreviation": abbreviation
        });

        Ok(result.to_string())
    }

    pub async fn list_timezones(arguments: Value) -> Result<String> {
        let region_filter = arguments.get("region")
            .and_then(|v| v.as_str());

        let timezones: Vec<String> = TZ_VARIANTS
            .iter()
            .map(|tz| tz.name().to_string())
            .filter(|name| {
                if let Some(region) = region_filter {
                    name.starts_with(region)
                } else {
                    true
                }
            })
            .collect();

        let result = json!({
            "timezones": timezones,
            "count": timezones.len()
        });

        Ok(result.to_string())
    }

    fn parse_timestamp(timestamp_str: &str) -> Result<DateTime<Utc>> {
        if let Ok(unix_timestamp) = timestamp_str.parse::<i64>() {
            DateTime::from_timestamp(unix_timestamp, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid Unix timestamp"))
        } else {
            DateTime::parse_from_rfc3339(timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| anyhow::anyhow!("Invalid timestamp format"))
        }
    }
}