use crate::amount::{Amount, Bps};
use crate::error::StratumResult;
use crate::snapshot::VaultSnapshot;
use crate::tranche::{TrancheKind, TranchePolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrancheTarget {
    pub tranche: TrancheKind,
    pub current: Amount,
    pub target: Amount,
    pub floor: Amount,
    pub weight: Bps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebalanceAction {
    Hold,
    RouteIn {
        tranche: TrancheKind,
        amount: Amount,
    },
    RouteOut {
        tranche: TrancheKind,
        amount: Amount,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalancePlan {
    pub total_reserve: Amount,
    pub targets: Vec<TrancheTarget>,
    pub actions: Vec<RebalanceAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalancePlanner {
    policy: TranchePolicy,
    dust: Amount,
}

impl TrancheTarget {
    pub fn drift(&self) -> i128 {
        self.current.units() as i128 - self.target.units() as i128
    }

    pub fn is_below_floor(&self) -> bool {
        self.current < self.floor
    }
}

impl RebalancePlan {
    pub fn action_for(&self, tranche: TrancheKind) -> RebalanceAction {
        self.actions
            .iter()
            .copied()
            .find(|action| match action {
                RebalanceAction::Hold => false,
                RebalanceAction::RouteIn {
                    tranche: action_tranche,
                    ..
                }
                | RebalanceAction::RouteOut {
                    tranche: action_tranche,
                    ..
                } => *action_tranche == tranche,
            })
            .unwrap_or(RebalanceAction::Hold)
    }

    pub fn total_route_in(&self) -> StratumResult<Amount> {
        self.actions.iter().try_fold(Amount::zero(), |acc, action| {
            let amount = match action {
                RebalanceAction::RouteIn { amount, .. } => *amount,
                RebalanceAction::Hold | RebalanceAction::RouteOut { .. } => Amount::zero(),
            };
            acc.checked_add(amount)
        })
    }

    pub fn total_route_out(&self) -> StratumResult<Amount> {
        self.actions.iter().try_fold(Amount::zero(), |acc, action| {
            let amount = match action {
                RebalanceAction::RouteOut { amount, .. } => *amount,
                RebalanceAction::Hold | RebalanceAction::RouteIn { .. } => Amount::zero(),
            };
            acc.checked_add(amount)
        })
    }
}

impl RebalancePlanner {
    pub fn new(policy: TranchePolicy, dust: Amount) -> Self {
        Self { policy, dust }
    }

    pub fn policy(&self) -> &TranchePolicy {
        &self.policy
    }

    pub const fn dust(&self) -> Amount {
        self.dust
    }

    pub fn plan(&self, snapshot: &VaultSnapshot) -> StratumResult<RebalancePlan> {
        let targets = self.targets(snapshot)?;
        let actions = targets
            .iter()
            .map(|target| self.action_for_target(*target))
            .collect();
        Ok(RebalancePlan {
            total_reserve: snapshot.total_reserve,
            targets,
            actions,
        })
    }

    pub fn targets(&self, snapshot: &VaultSnapshot) -> StratumResult<Vec<TrancheTarget>> {
        let target_amounts = self.policy.target_amounts(snapshot.total_reserve)?;
        let mut targets = Vec::new();
        for kind in TrancheKind::public_order() {
            let current = snapshot
                .tranche(kind)
                .map(|tranche| tranche.reserve)
                .unwrap_or_else(Amount::zero);
            let target = target_amounts
                .get(&kind)
                .copied()
                .unwrap_or_else(Amount::zero);
            let floor = self.policy.required_floor(kind, snapshot.total_reserve)?;
            targets.push(TrancheTarget {
                tranche: kind,
                current,
                target,
                floor,
                weight: self.policy.target_weight(kind),
            });
        }
        Ok(targets)
    }

    fn action_for_target(&self, target: TrancheTarget) -> RebalanceAction {
        if target.current < target.floor {
            return RebalanceAction::RouteIn {
                tranche: target.tranche,
                amount: target.floor.saturating_sub(target.current),
            };
        }
        if target.current < target.target {
            let deficit = target.target.saturating_sub(target.current);
            if deficit > self.dust {
                return RebalanceAction::RouteIn {
                    tranche: target.tranche,
                    amount: deficit,
                };
            }
        }
        if target.current > target.target {
            let surplus = target.current.saturating_sub(target.target);
            if surplus > self.dust {
                return RebalanceAction::RouteOut {
                    tranche: target.tranche,
                    amount: surplus,
                };
            }
        }
        RebalanceAction::Hold
    }
}

impl Default for RebalancePlanner {
    fn default() -> Self {
        Self::new(TranchePolicy::balanced(), Amount::from_units(10))
    }
}
