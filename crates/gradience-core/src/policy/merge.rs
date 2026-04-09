use crate::policy::engine::Policy;
use crate::policy::engine::Rule;

#[derive(Debug, Clone, Default)]
pub struct MergedPolicy {
    pub spend_limit: Option<String>,
    pub daily_limit: Option<String>,
    pub monthly_limit: Option<String>,
    pub shared_budget: Option<String>,
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
    let all: Vec<&Policy> = workspace.into_iter().chain(agents).collect();

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

    // monthly_limit: min
    let monthly: Vec<String> = all.iter()
        .flat_map(|p| p.rules.iter())
        .filter_map(|r| match r { Rule::MonthlyLimit { max, .. } => Some(max.clone()), _ => None })
        .collect();
    merged.monthly_limit = min_str(monthly);

    // shared_budget: min
    let shared: Vec<String> = all.iter()
        .flat_map(|p| p.rules.iter())
        .filter_map(|r| match r { Rule::SharedBudget { max, .. } => Some(max.clone()), _ => None })
        .collect();
    merged.shared_budget = min_str(shared);

    // contract_whitelist: intersection
    let contract_lists: Vec<Vec<String>> = all.iter()
        .flat_map(|p| p.rules.iter())
        .filter_map(|r| match r { Rule::ContractWhitelist { contracts } => Some(contracts.clone()), _ => None })
        .collect();
    merged.contract_whitelist = intersect_vec(contract_lists);

    // operation_type: intersection
    let op_lists: Vec<Vec<String>> = all.iter()
        .flat_map(|p| p.rules.iter())
        .filter_map(|r| match r { Rule::OperationType { allowed } => Some(allowed.clone()), _ => None })
        .collect();
    merged.operation_type = intersect_vec(op_lists);

    // time_window: narrowest
    let windows: Vec<TimeWindowRule> = all.iter()
        .flat_map(|p| p.rules.iter())
        .filter_map(|r| match r {
            Rule::TimeWindow { start_hour, end_hour, timezone } => {
                Some(TimeWindowRule { start_hour: *start_hour, end_hour: *end_hour, timezone: timezone.clone() })
            }
            _ => None
        })
        .collect();
    merged.time_window = narrowest_window(windows);

    // max_tokens: min
    let tokens: Vec<u64> = all.iter()
        .flat_map(|p| p.rules.iter())
        .filter_map(|r| match r { Rule::MaxTokensPerCall { limit } => Some(*limit), _ => None })
        .collect();
    merged.max_tokens = tokens.into_iter().min();

    merged
}

fn narrowest_window(windows: Vec<TimeWindowRule>) -> Option<TimeWindowRule> {
    if windows.is_empty() {
        return None;
    }
    // Narrowest = smallest duration
    windows.into_iter().min_by(|a, b| {
        let dur_a = if a.start_hour <= a.end_hour {
            a.end_hour - a.start_hour
        } else {
            24 - a.start_hour + a.end_hour
        };
        let dur_b = if b.start_hour <= b.end_hour {
            b.end_hour - b.start_hour
        } else {
            24 - b.start_hour + b.end_hour
        };
        dur_a.cmp(&dur_b)
    })
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
