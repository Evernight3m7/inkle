use std::env;

#[derive(Debug, Clone)]
pub struct EnvConfig {
    pub admin_password: String,
    pub jwt_secret: String,
    pub database_url: String,
    pub ai_base_url: String,
    pub ai_api_key: String,
}

impl EnvConfig {
    pub fn from_env() -> Self {
        // Load .env file if present; ignore error if not found
        let _ = dotenvy::dotenv();

        Self {
            admin_password: {
                let pw = env::var("ADMIN_PASSWORD")
                    .expect("ADMIN_PASSWORD environment variable is required");
                assert!(!pw.is_empty(), "ADMIN_PASSWORD must not be empty");
                pw
            },
            jwt_secret: env::var("JWT_SECRET")
                .expect("JWT_SECRET environment variable is required"),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string()),
            ai_base_url: env::var("AI_BASE_URL").unwrap_or_default(),
            ai_api_key: env::var("AI_API_KEY").unwrap_or_default(),
        }
    }
}
