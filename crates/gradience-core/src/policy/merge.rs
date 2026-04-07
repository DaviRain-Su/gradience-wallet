use crate::policy::engine::Policy;
use crate::policy::engine::Rule;

#[derive(Debug, Clone, Default)]
pub struct MergedPolicy {
    pub spend_limit: Option<String>,
    pub daily_limit: Option<String>,
    pub monthly_limit: Option<String>,
    pub chain_whitelist: Option<Vec<String>>,
    pub contract_whitelist: Option<Vec<String>>,
    pub operation_type: Option<Vec<String>>,
    pub time_window: Option<TimeWindowRule>,
    pub max_tokens: Option<u64>,
    pub model_whitelist: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct TimeWindowRule {
    pub start_hour: u8,
    pub end_hour: u8,
    pub timezone: String,
}

pub fn merge_policies_strictest(
    workspace: Option<&Policy>,
    agents: Vec<&Policy>,
) -> MergedPolicy {
    let mut merged = MergedPolicy::default();
    let all: Vec<&Policy> = workspace.into_iter().chain(agents.into_iter()).collect();

    // spend_limit: min
    let limits: Vec<String> = all.iter()
        .flat_map(|p| p.rules.iter())
        .filter_map(|r| match r { Rule::SpendLimit { max, .. } => Some(max.clone()), _ => None })
        .collect();
    merged.spend_limit = min_str(limits);

    // daily_limit: min
    let daily: Vec<String> = all.iter()
        .flat_map(|p| p.rules.iter())
        .filter_map(|r| match r { Rule::DailyLimit { max, .. } => Some(max.clone()), _ => None })
        .collect();
    merged.daily_limit = min_str(daily);

    // chain_whitelist: intersection
    let chain_lists: Vec<Vec<String>> = all.iter()
        .flat_map(|p| p.rules.iter())
        .filter_map(|r| match r { Rule::ChainWhitelist { chain_ids } => Some(chain_ids.clone()), _ => None })
        .collect();
    merged.chain_whitelist = intersect_vec(chain_lists);

    // model_whitelist: intersection
    let model_lists: Vec<Vec<String>> = all.iter()
        .flat_map(|p| p.rules.iter())
        .filter_map(|r| match r { Rule::ModelWhitelist { models } => Some(models.clone()), _ => None })
        .collect();
    merged.model_whitelist = intersect_vec(model_lists);

    merged
}

fn min_str(vals: Vec<String>) -> Option<String> {
    vals.into_iter().min_by(|a, b| {
        let na = a.parse::<u64>().unwrap_or(u64::MAX);
        let nb = b.parse::<u64>().unwrap_or(u64::MAX);
        na.cmp(&nb)
    })
}

fn intersect_vec(lists: Vec<Vec<String>>) -> Option<Vec<String>> {
    if lists.is_empty() {
        return None;
    }
    let mut result = lists[0].clone();
    for list in &lists[1..] {
        result.retain(|x| list.contains(x));
    }
    Some(result)
}
