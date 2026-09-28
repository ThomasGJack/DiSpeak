use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};
use uuid::Uuid;

pub const DEFAULT_CONFIG_PATH: &str = "server-config.toml";

pub const CONFIG_EXAMPLE: &str = r#"# DiSpeak Server Binary Configuration

[server]
id = "uuid-du-serveur-cree-dans-le-backend"
database_url = "sqlite:server.db"
ws_host = "0.0.0.0"
ws_port = 8001
internal_api_host = "127.0.0.1"
internal_api_port = 8002
instance_name = "Mon serveur DiSpeak"
internal_api_token = "change-me-internal-api-token"

[backend]
url = "https://backend.example.com"
owner_key = "dso_xxxxx_xxxxxxxxx"
bootstrap_timeout_ms = 5000
heartbeat_interval_sec = 30

[security]
enforce_owner_validation = true
allow_insecure_http_backend = false
allow_legacy_hs256 = false

[logging]
level = "info"
json = false
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Run { config_path: Option<PathBuf> },
    ValidateConfig { config_path: Option<PathBuf> },
    PrintConfigExample,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TomlConfig {
    pub server: Option<ServerSection>,
    pub backend: Option<BackendSection>,
    pub security: Option<SecuritySection>,
    pub logging: Option<LoggingSection>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ServerSection {
    pub id: Option<String>,
    pub database_url: Option<String>,
    pub ws_host: Option<String>,
    pub ws_port: Option<u16>,
    pub internal_api_host: Option<String>,
    pub internal_api_port: Option<u16>,
    pub instance_name: Option<String>,
    pub internal_api_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BackendSection {
    pub url: Option<String>,
    pub owner_key: Option<String>,
    pub bootstrap_timeout_ms: Option<u64>,
    pub heartbeat_interval_sec: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SecuritySection {
    pub enforce_owner_validation: Option<bool>,
    pub allow_insecure_http_backend: Option<bool>,
    pub allow_legacy_hs256: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct LoggingSection {
    pub level: Option<String>,
    pub json: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub config_path_used: Option<PathBuf>,
    pub server_id: String,
    pub database_url: String,
    pub ws_host: String,
    pub ws_port: u16,
    pub internal_api_host: String,
    pub internal_api_port: Option<u16>,
    pub internal_api_token: Option<String>,
    pub instance_name: Option<String>,
    pub backend_url: String,
    pub owner_key: Option<String>,
    pub bootstrap_timeout_ms: u64,
    pub heartbeat_interval_sec: u64,
    pub enforce_owner_validation: bool,
    pub allow_insecure_http_backend: bool,
    pub allow_legacy_hs256: bool,
    pub jwt_secret: Option<String>,
    pub legacy_auth_token: Option<String>,
}

impl RuntimeConfig {
    pub fn should_use_bootstrap(&self) -> bool {
        self.enforce_owner_validation && self.owner_key.as_ref().map(|v| !v.trim().is_empty()).unwrap_or(false)
    }

    pub fn resolved_internal_api_port(&self) -> u16 {
        self.internal_api_port.unwrap_or(self.ws_port.saturating_add(1))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.server_id.trim().is_empty() {
            return Err("server.id / SERVER_ID manquant".to_string());
        }

        if Uuid::parse_str(self.server_id.trim()).is_err() {
            return Err("server.id / SERVER_ID doit être un UUID valide".to_string());
        }

        if self.database_url.trim().is_empty() {
            return Err("server.database_url / DATABASE_URL manquant".to_string());
        }

        if self.backend_url.trim().is_empty() {
            return Err("backend.url / BACKEND_URL manquant".to_string());
        }

        if self.should_use_bootstrap() {
            let owner_key = self.owner_key.as_deref().unwrap_or_default();
            if !owner_key.starts_with("dso_") {
                return Err("backend.owner_key / OWNER_KEY doit commencer par dso_".to_string());
            }
        }

        if self.should_use_bootstrap()
            && !self.allow_insecure_http_backend
            && self.backend_url.starts_with("http://")
        {
            return Err(
                "backend.url en HTTP refusé (activez security.allow_insecure_http_backend=true pour le dev)"
                    .to_string(),
            );
        }

        Ok(())
    }
}

pub fn parse_cli_args() -> Result<CliCommand, String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return Ok(CliCommand::Run { config_path: None });
    }

    let command = args.remove(0);
    let mut config_path: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                let next = args.get(i + 1).ok_or("--config requiert un chemin".to_string())?;
                config_path = Some(PathBuf::from(next));
                i += 2;
            }
            other => {
                return Err(format!("Argument inconnu: {other}"));
            }
        }
    }

    match command.as_str() {
        "run" => Ok(CliCommand::Run { config_path }),
        "validate-config" => Ok(CliCommand::ValidateConfig { config_path }),
        "print-config-example" => Ok(CliCommand::PrintConfigExample),
        _ => Err(format!(
            "Commande inconnue: {} (attendu: run | validate-config | print-config-example)",
            command
        )),
    }
}

pub fn load_runtime_config(config_path: Option<PathBuf>) -> Result<RuntimeConfig, String> {
    let explicit_path = config_path;
    let default_path = PathBuf::from(DEFAULT_CONFIG_PATH);

    let selected_path = explicit_path.clone().or_else(|| {
        if default_path.exists() {
            Some(default_path.clone())
        } else {
            None
        }
    });

    let toml_config = if let Some(path) = selected_path.as_ref() {
        let content = fs::read_to_string(path)
            .map_err(|err| format!("Impossible de lire {}: {err}", path.display()))?;
        Some(
            toml::from_str::<TomlConfig>(&content)
                .map_err(|err| format!("TOML invalide ({}): {err}", path.display()))?,
        )
    } else {
        None
    };

    let toml_server = toml_config.as_ref().and_then(|c| c.server.as_ref());
    let toml_backend = toml_config.as_ref().and_then(|c| c.backend.as_ref());
    let toml_security = toml_config.as_ref().and_then(|c| c.security.as_ref());

    let server_id = toml_server
        .and_then(|s| s.id.clone())
        .or_else(|| env::var("SERVER_ID").ok())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let runtime = RuntimeConfig {
        config_path_used: selected_path.clone(),
        server_id,
        database_url: toml_server
            .and_then(|s| s.database_url.clone())
            .or_else(|| env::var("DATABASE_URL").ok())
            .unwrap_or_else(|| "sqlite:server.db".to_string()),
        ws_host: toml_server
            .and_then(|s| s.ws_host.clone())
            .or_else(|| env::var("SERVER_HOST").ok())
            .unwrap_or_else(|| "127.0.0.1".to_string()),
        ws_port: toml_server
            .and_then(|s| s.ws_port)
            .or_else(|| env::var("SERVER_PORT").ok().and_then(|v| v.parse::<u16>().ok()))
            .unwrap_or(8001),
        internal_api_host: toml_server
            .and_then(|s| s.internal_api_host.clone())
            .or_else(|| env::var("SERVER_INTERNAL_API_HOST").ok())
            .unwrap_or_else(|| "127.0.0.1".to_string()),
        internal_api_port: toml_server
            .and_then(|s| s.internal_api_port)
            .or_else(|| {
                env::var("SERVER_INTERNAL_API_PORT")
                    .ok()
                    .and_then(|v| v.parse::<u16>().ok())
            }),
        internal_api_token: toml_server
            .and_then(|s| s.internal_api_token.clone())
            .or_else(|| env::var("SERVER_INTERNAL_API_TOKEN").ok())
            .filter(|v| !v.trim().is_empty()),
        instance_name: toml_server
            .and_then(|s| s.instance_name.clone())
            .filter(|v| !v.trim().is_empty()),
        backend_url: toml_backend
            .and_then(|b| b.url.clone())
            .or_else(|| env::var("BACKEND_URL").ok())
            .unwrap_or_else(|| "http://127.0.0.1:8000".to_string()),
        owner_key: toml_backend
            .and_then(|b| b.owner_key.clone())
            .or_else(|| env::var("OWNER_KEY").ok())
            .filter(|v| !v.trim().is_empty()),
        bootstrap_timeout_ms: toml_backend
            .and_then(|b| b.bootstrap_timeout_ms)
            .unwrap_or_else(|| {
                env::var("SERVER_BOOTSTRAP_TIMEOUT_MS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(5000)
            }),
        heartbeat_interval_sec: toml_backend
            .and_then(|b| b.heartbeat_interval_sec)
            .unwrap_or_else(|| {
                env::var("SERVER_HOSTING_HEARTBEAT_INTERVAL_SEC")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(30)
            }),
        enforce_owner_validation: toml_security
            .and_then(|s| s.enforce_owner_validation)
            .unwrap_or_else(|| {
                env::var("SERVER_ENFORCE_OWNER_VALIDATION")
                    .ok()
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(true)
            }),
        allow_insecure_http_backend: toml_security
            .and_then(|s| s.allow_insecure_http_backend)
            .unwrap_or_else(|| {
                env::var("SERVER_ALLOW_INSECURE_HTTP_BACKEND")
                    .ok()
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false)
            }),
        allow_legacy_hs256: toml_security
            .and_then(|s| s.allow_legacy_hs256)
            .unwrap_or_else(|| {
                env::var("SERVER_ALLOW_LEGACY_HS256")
                    .ok()
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false)
            }),
        jwt_secret: env::var("JWT_SECRET").ok().filter(|v| !v.trim().is_empty()),
        legacy_auth_token: env::var("AUTH_TOKEN").ok().filter(|v| !v.trim().is_empty()),
    };

    runtime.validate()?;
    Ok(runtime)
}
