use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    User,
    Viewer,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub blocked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateUserParams<'a> {
    pub email: &'a str,
    pub password_hash: &'a str,
    pub role: UserRole,
    pub blocked: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateUserParams<'a> {
    pub email: Option<&'a str>,
    pub password_hash: Option<&'a str>,
    pub role: Option<UserRole>,
    pub blocked: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct UserListParams {
    pub limit: i64,
    pub cursor: Option<UserListCursor>,
}

#[derive(Debug, Clone)]
pub struct UserListCursor {
    pub created_at: DateTime<Utc>,
    pub user_id: Uuid,
}

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<UserRecord>, sqlx::Error> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            SELECT id, email, password_hash, role, blocked, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<UserRecord>, sqlx::Error> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            SELECT id, email, password_hash, role, blocked, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list(&self, params: UserListParams) -> Result<Vec<UserRecord>, sqlx::Error> {
        if let Some(UserListCursor {
            created_at,
            user_id,
        }) = params.cursor
        {
            sqlx::query_as::<_, UserRecord>(
                r#"
                SELECT id, email, password_hash, role, blocked, created_at, updated_at
                FROM users
                WHERE (created_at, id) < ($2, $3)
                ORDER BY created_at DESC, id DESC
                LIMIT $1
                "#,
            )
            .bind(params.limit)
            .bind(created_at)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, UserRecord>(
                r#"
                SELECT id, email, password_hash, role, blocked, created_at, updated_at
                FROM users
                ORDER BY created_at DESC, id DESC
                LIMIT $1
                "#,
            )
            .bind(params.limit)
            .fetch_all(&self.pool)
            .await
        }
    }

    pub async fn create(&self, params: CreateUserParams<'_>) -> Result<UserRecord, sqlx::Error> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            INSERT INTO users (email, password_hash, role, blocked)
            VALUES ($1, $2, $3::user_role, $4)
            RETURNING id, email, password_hash, role, blocked, created_at, updated_at
            "#,
        )
        .bind(params.email)
        .bind(params.password_hash)
        .bind(params.role)
        .bind(params.blocked)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update(
        &self,
        user_id: Uuid,
        params: UpdateUserParams<'_>,
    ) -> Result<UserRecord, sqlx::Error> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            UPDATE users
            SET email = COALESCE($2, email),
                password_hash = COALESCE($3, password_hash),
                role = COALESCE($4::user_role, role),
                blocked = COALESCE($5, blocked),
                updated_at = now()
            WHERE id = $1
            RETURNING id, email, password_hash, role, blocked, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(params.email)
        .bind(params.password_hash)
        .bind(params.role)
        .bind(params.blocked)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn ensure_admin(
        &self,
        email: &str,
        password_hash: &str,
    ) -> Result<UserRecord, sqlx::Error> {
        if let Some(existing) = self.find_by_email(email).await? {
            // Ensure the admin user remains active and has the correct role, but do not override password.
            let updated = self
                .update(
                    existing.id,
                    UpdateUserParams {
                        email: None,
                        password_hash: None,
                        role: Some(UserRole::Admin),
                        blocked: Some(false),
                    },
                )
                .await?;
            Ok(updated)
        } else {
            self.create(CreateUserParams {
                email,
                password_hash,
                role: UserRole::Admin,
                blocked: false,
            })
            .await
        }
    }
}
