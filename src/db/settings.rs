use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    sqlx::Type,
    utoipa::ToSchema,
)]
#[sqlx(type_name = "user_theme", rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum UserTheme {
    Dark,
}

#[derive(Clone)]
pub struct UserSettingsRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserSettings {
    pub user_id: Uuid,
    pub default_k: Option<i32>,
    pub default_lang: Option<String>,
    pub theme: Option<UserTheme>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpdateUserSettingsParams {
    pub default_k: Option<i32>,
    pub default_lang: Option<String>,
    pub theme: Option<UserTheme>,
}

impl UserSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, user_id: Uuid) -> Result<Option<UserSettings>, sqlx::Error> {
        sqlx::query_as!(
            UserSettings,
            r#"
            SELECT
                user_id,
                default_k,
                default_lang,
                theme as "theme: UserTheme",
                created_at,
                updated_at
            FROM user_settings
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn upsert(
        &self,
        user_id: Uuid,
        params: UpdateUserSettingsParams,
    ) -> Result<UserSettings, sqlx::Error> {
        sqlx::query_as!(
            UserSettings,
            r#"
            INSERT INTO user_settings (
                user_id,
                default_k,
                default_lang,
                theme
            )
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE SET
                default_k = COALESCE(EXCLUDED.default_k, user_settings.default_k),
                default_lang = COALESCE(EXCLUDED.default_lang, user_settings.default_lang),
                theme = COALESCE(EXCLUDED.theme, user_settings.theme),
                updated_at = NOW()
            RETURNING
                user_id,
                default_k,
                default_lang,
                theme as "theme: UserTheme",
                created_at,
                updated_at
            "#,
            user_id,
            params.default_k,
            params.default_lang,
            params.theme as _
        )
        .fetch_one(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::UserTheme;

    #[test]
    fn user_theme_rejects_unknown_value() {
        let value = serde_json::from_str::<UserTheme>("\"light\"");

        assert!(value.is_err());
    }
}
