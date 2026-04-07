use crate::team::workspace::{WorkspaceService, WorkspaceRole, WorkspaceDescriptor};
use crate::error::GradienceError;

#[test]
fn test_create_workspace_success() {
    let svc = WorkspaceService::new();
    let ws = svc.create_workspace("hackathon-team", "user-1").unwrap();
    assert_eq!(ws.name, "hackathon-team");
    assert_eq!(ws.owner_id, "user-1");
    assert_eq!(ws.plan, "free");
}

#[test]
fn test_create_workspace_empty_name_fails() {
    let svc = WorkspaceService::new();
    let err = svc.create_workspace("  ", "user-1").unwrap_err();
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

#[test]
fn test_add_member() {
    let svc = WorkspaceService::new();
    let member = svc.add_member("ws-1", "user-2", WorkspaceRole::Member).unwrap();
    assert_eq!(member.workspace_id, "ws-1");
    assert_eq!(member.user_id, "user-2");
    assert_eq!(member.role, WorkspaceRole::Member);
}

#[test]
fn test_role_from_str() {
    assert_eq!(WorkspaceRole::from_str("owner").unwrap(), WorkspaceRole::Owner);
    assert_eq!(WorkspaceRole::from_str("admin").unwrap(), WorkspaceRole::Admin);
    assert_eq!(WorkspaceRole::from_str("member").unwrap(), WorkspaceRole::Member);
    assert_eq!(WorkspaceRole::from_str("viewer").unwrap(), WorkspaceRole::Viewer);
    assert!(WorkspaceRole::from_str("hacker").is_err());
}
