use sqlx::PgPool;
use uuid::Uuid;

use super::users::UserRole;

#[derive(Clone)]
pub struct ContextShareRepository {
    pool: PgPool,
}

impl ContextShareRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn is_shared_with(
        &self,
        context_id: i32,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT 1
            FROM context_shares
            WHERE context_id = $1 AND user_id = $2
            "#,
        )
        .bind(context_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(exists.is_some())
    }

    pub async fn create_share(
        &self,
        context_id: i32,
        user_id: Uuid,
        role: UserRole,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO context_shares (context_id, user_id, role)
            VALUES ($1, $2, $3::user_role)
            ON CONFLICT (context_id, user_id) DO UPDATE SET role = EXCLUDED.role
            "#,
        )
        .bind(context_id)
        .bind(user_id)
        .bind(role)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_share(&self, context_id: i32, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            DELETE FROM context_shares
            WHERE context_id = $1 AND user_id = $2
            "#,
            context_id,
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
