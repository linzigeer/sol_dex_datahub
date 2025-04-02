use borsh::BorshDeserialize;

pub mod accounts;
pub mod event;
pub mod instruction;

#[derive(Debug, BorshDeserialize, Clone, Copy, PartialEq)]
pub enum MeteoraDammPoolType {
    /// Permissioned
    Permissioned,
    /// Permissionless
    Permissionless,
}
