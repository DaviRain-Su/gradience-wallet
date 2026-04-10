use crate::ows::adapter::Transaction;
use crate::policy::engine::{Decision, EvalContext, Policy, PolicyEngine, Rule};
use crate::policy::merge::merge_policies_strictest;

fn default_policy() -> Policy {
    Policy {
        id: "p1".into(),
        name: "test".into(),
        wallet_id: None,
        workspace_id: None,
        rules: vec![],
        priority: 1,
        status: "active".into(),
        version: 1,
        created_at: "".into(),
        updated_at: "".into(),
    }
}

fn make_ctx(chain: &str, value: &str) -> EvalContext {
    EvalContext {
        wallet_id: "w1".into(),
        api_key_id: "k1".into(),
        chain_id: chain.into(),
        transaction: Transaction {
            to: None,
            value: value.into(),
            data: vec![],
            raw_hex: "0x".into(),
        },
        intent: None,
        timestamp_ms: 0,
        dynamic_signals: None,
        max_tokens: None,
        model: None,
        session_id: None,
    }
}

#[test]
fn test_chain_whitelist_allow() {
    let engine = PolicyEngine;
    let policy = Policy {
        rules: vec![Rule::ChainWhitelist {
            chain_ids: vec!["eip155:8453".into(), "eip155:56".into()],
        }],
        ..default_policy()
    };
    let ctx = make_ctx("eip155:8453", "0");
    let result = engine.evaluate(ctx, vec![&policy]).unwrap();
    assert_eq!(result.decision, Decision::Allow);
}

#[test]
fn test_chain_whitelist_deny() {
    let engine = PolicyEngine;
    let policy = Policy {
        rules: vec![Rule::ChainWhitelist {
            chain_ids: vec!["eip155:8453".into()],
        }],
        ..default_policy()
    };
    let ctx = make_ctx("eip155:1", "0");
    let result = engine.evaluate(ctx, vec![&policy]).unwrap();
    assert_eq!(result.decision, Decision::Deny);
}

#[test]
fn test_spend_limit_allow_under_threshold() {
    let engine = PolicyEngine;
    let policy = Policy {
        rules: vec![Rule::SpendLimit {
            max: "1000".into(),
            token: "USDC".into(),
        }],
        ..default_policy()
    };
    let ctx = make_ctx("eip155:8453", "700");
    let result = engine.evaluate(ctx, vec![&policy]).unwrap();
    assert_eq!(result.decision, Decision::Allow);
}

#[test]
fn test_spend_limit_warn_at_threshold() {
    let engine = PolicyEngine;
    let policy = Policy {
        rules: vec![Rule::SpendLimit {
            max: "1000".into(),
            token: "USDC".into(),
        }],
        ..default_policy()
    };
    let ctx = make_ctx("eip155:8453", "900");
    let result = engine.evaluate(ctx, vec![&policy]).unwrap();
    assert_eq!(result.decision, Decision::Warn);
}

#[test]
fn test_spend_limit_deny_over_boundary() {
    let engine = PolicyEngine;
    let policy = Policy {
        rules: vec![Rule::SpendLimit {
            max: "1000".into(),
            token: "USDC".into(),
        }],
        ..default_policy()
    };
    let ctx = make_ctx("eip155:8453", "1001");
    let result = engine.evaluate(ctx, vec![&policy]).unwrap();
    assert_eq!(result.decision, Decision::Deny);
}

#[test]
fn test_spend_limit_zero_attack() {
    let engine = PolicyEngine;
    let policy = Policy {
        rules: vec![Rule::SpendLimit {
            max: "0".into(),
            token: "USDC".into(),
        }],
        ..default_policy()
    };
    let ctx = make_ctx("eip155:8453", "1");
    let result = engine.evaluate(ctx, vec![&policy]).unwrap();
    assert_eq!(result.decision, Decision::Deny);
}

#[test]
fn test_merge_spend_limit_takes_min() {
    let wp = Policy {
        rules: vec![Rule::SpendLimit {
            max: "1000".into(),
            token: "USDC".into(),
        }],
        priority: 0,
        ..default_policy()
    };
    let ap = Policy {
        rules: vec![Rule::SpendLimit {
            max: "500".into(),
            token: "USDC".into(),
        }],
        priority: 1,
        ..default_policy()
    };
    let merged = merge_policies_strictest(Some(&wp), vec![&ap]);
    assert_eq!(merged.spend_limit, Some("500".into()));
}

#[test]
fn test_merge_chain_whitelist_intersection() {
    let wp = Policy {
        rules: vec![Rule::ChainWhitelist {
            chain_ids: vec!["eip155:8453".into(), "eip155:56".into(), "eip155:1".into()],
        }],
        priority: 0,
        ..default_policy()
    };
    let ap = Policy {
        rules: vec![Rule::ChainWhitelist {
            chain_ids: vec!["eip155:8453".into(), "eip155:56".into()],
        }],
        priority: 1,
        ..default_policy()
    };
    let merged = merge_policies_strictest(Some(&wp), vec![&ap]);
    assert_eq!(
        merged.chain_whitelist,
        Some(vec!["eip155:8453".into(), "eip155:56".into()])
    );
}

#[test]
fn test_merge_empty_intersection_deny_all() {
    let wp = Policy {
        rules: vec![Rule::ChainWhitelist {
            chain_ids: vec!["eip155:1".into()],
        }],
        priority: 0,
        ..default_policy()
    };
    let ap = Policy {
        rules: vec![Rule::ChainWhitelist {
            chain_ids: vec!["eip155:8453".into()],
        }],
        priority: 1,
        ..default_policy()
    };
    let merged = merge_policies_strictest(Some(&wp), vec![&ap]);
    assert_eq!(merged.chain_whitelist, Some(vec![]));
}
