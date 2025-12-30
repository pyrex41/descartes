use crate::types::{Project, CreateProjectRequest};
use chrono::Utc;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

/// SQLite-backed project storage
pub struct ProjectStore {
    pool: SqlitePool,
}

impl ProjectStore {
    /// Create a new ProjectStore and run migrations
    pub async fn new(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        // Run migrations
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                prd_content TEXT,
                scud_tag TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#)
        .execute(&pool)
        .await?;

        // Create index for owner lookups
        sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_projects_owner ON projects(owner_id)
        "#)
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    /// Create a new project
    pub async fn create(&self, owner_id: &str, req: CreateProjectRequest) -> Result<Project, sqlx::Error> {
        let now = Utc::now();
        let project = Project {
            id: Uuid::new_v4().to_string(),
            name: req.name,
            owner_id: owner_id.to_string(),
            prd_content: req.prd_content,
            scud_tag: None,
            created_at: now,
            updated_at: now,
        };

        sqlx::query(r#"
            INSERT INTO projects (id, name, owner_id, prd_content, scud_tag, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&project.id)
        .bind(&project.name)
        .bind(&project.owner_id)
        .bind(&project.prd_content)
        .bind(&project.scud_tag)
        .bind(project.created_at.to_rfc3339())
        .bind(project.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(project)
    }

    /// List all projects for an owner
    pub async fn list(&self, owner_id: &str) -> Result<Vec<Project>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ProjectRow>(
            "SELECT id, name, owner_id, prd_content, scud_tag, created_at, updated_at FROM projects WHERE owner_id = ? ORDER BY updated_at DESC"
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_project()).collect())
    }

    /// Get a project by ID
    pub async fn get(&self, id: &str) -> Result<Option<Project>, sqlx::Error> {
        let row = sqlx::query_as::<_, ProjectRow>(
            "SELECT id, name, owner_id, prd_content, scud_tag, created_at, updated_at FROM projects WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_project()))
    }

    /// Delete a project by ID
    pub async fn delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Update a project's PRD content
    pub async fn update_prd(&self, id: &str, prd_content: &str) -> Result<bool, sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query("UPDATE projects SET prd_content = ?, updated_at = ? WHERE id = ?")
            .bind(prd_content)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// Helper struct for SQLite row mapping
#[derive(FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    owner_id: String,
    prd_content: Option<String>,
    scud_tag: Option<String>,
    created_at: String,
    updated_at: String,
}

impl ProjectRow {
    fn into_project(self) -> Project {
        use chrono::DateTime;
        Project {
            id: self.id,
            name: self.name,
            owner_id: self.owner_id,
            prd_content: self.prd_content,
            scud_tag: self.scud_tag,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get_project() {
        let pool = setup_test_db().await;
        let store = ProjectStore::new(pool).await.unwrap();

        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            prd_content: Some("# PRD\nThis is a test".to_string()),
        };

        let project = store.create("user123", req).await.unwrap();
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.owner_id, "user123");

        let retrieved = store.get(&project.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, project.id);
    }

    #[tokio::test]
    async fn test_list_projects() {
        let pool = setup_test_db().await;
        let store = ProjectStore::new(pool).await.unwrap();

        store.create("user1", CreateProjectRequest { name: "P1".to_string(), prd_content: None }).await.unwrap();
        store.create("user1", CreateProjectRequest { name: "P2".to_string(), prd_content: None }).await.unwrap();
        store.create("user2", CreateProjectRequest { name: "P3".to_string(), prd_content: None }).await.unwrap();

        let user1_projects = store.list("user1").await.unwrap();
        assert_eq!(user1_projects.len(), 2);

        let user2_projects = store.list("user2").await.unwrap();
        assert_eq!(user2_projects.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_project() {
        let pool = setup_test_db().await;
        let store = ProjectStore::new(pool).await.unwrap();

        let project = store.create("user1", CreateProjectRequest { name: "ToDelete".to_string(), prd_content: None }).await.unwrap();

        let deleted = store.delete(&project.id).await.unwrap();
        assert!(deleted);

        let retrieved = store.get(&project.id).await.unwrap();
        assert!(retrieved.is_none());
    }
}
