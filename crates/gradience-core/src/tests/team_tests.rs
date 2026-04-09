use crate::team::workspace::{WorkspaceService, WorkspaceRole};
use crate::error::GradienceError;

async fn setup_db() -> sqlx::SqlitePool {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let _: Result<(), _> = sqlx::migrate!("../gradience-db/migrations").run(&db).await;
    sqlx::query("INSERT INTO users (id, email, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
        .bind("user-1")
        .bind("test@example.com")
        .bind("active")
        .bind(chrono::Utc::now())
        .bind(chrono::Utc::now())
        .execute(&db)
        .await
        .unwrap();
    db
}

#[tokio::test]
async fn test_create_workspace_success() {
    let db = setup_db().await;
    let svc = WorkspaceService::new();
    let id = svc.create_workspace(&db, "development-team", "user-1").await.unwrap();
    assert!(!id.is_empty());
}

#[tokio::test]
async fn test_create_workspace_empty_name_fails() {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let svc = WorkspaceService::new();
    let err = svc.create_workspace(&db, "  ", "user-1").await.unwrap_err();
    assert!(matches!(err, GradienceError::InvalidCredential(_)));
}

#[test]
fn test_workspace_role_permissions() {
    let svc = WorkspaceService::new();
    assert!(svc.check_role_permission(&WorkspaceRole::Owner, "set_policy"));
    assert!(svc.check_role_permission(&WorkspaceRole::Admin, "invite_member"));
    assert!(!svc.check_role_permission(&WorkspaceRole::Member, "set_policy"));
    assert!(!svc.check_role_permission(&WorkspaceRole::Viewer, "create_wallet"));
    assert!(svc.check_role_permission(&WorkspaceRole::Viewer, "view_balance"));
}

#[tokio::test]
async fn test_add_member() {
    let db = setup_db().await;
    sqlx::query("INSERT INTO users (id, email, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
        .bind("user-2")
        .bind("user2@example.com")
        .bind("active")
        .bind(chrono::Utc::now())
        .bind(chrono::Utc::now())
        .execute(&db)
        .await
        .unwrap();
    let svc = WorkspaceService::new();
    let ws_id = svc.create_workspace(&db, "team", "user-1").await.unwrap();
    svc.add_member(&db, &ws_id, "user-2", WorkspaceRole::Member).await.unwrap();
    let members = gradience_db::queries::list_workspace_members(&db, &ws_id).await.unwrap();
    assert_eq!(members.len(), 2); // owner + member
}

#[test]
fn test_role_from_str() {
    assert_eq!("owner".parse::<WorkspaceRole>().unwrap(), WorkspaceRole::Owner);
    assert_eq!("admin".parse::<WorkspaceRole>().unwrap(), WorkspaceRole::Admin);
    assert_eq!("member".parse::<WorkspaceRole>().unwrap(), WorkspaceRole::Member);
    assert_eq!("viewer".parse::<WorkspaceRole>().unwrap(), WorkspaceRole::Viewer);
    assert!("hacker".parse::<WorkspaceRole>().is_err());
}
